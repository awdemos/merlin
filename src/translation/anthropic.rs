use crate::protocol::{LlmResponse, Request, StopReason, Usage, WireFormat};
use crate::translation::WireCodec;

#[derive(Debug, Default)]
pub struct AnthropicMessagesCodec;

impl WireCodec for AnthropicMessagesCodec {
    fn wire_format(&self) -> WireFormat {
        WireFormat::AnthropicMessages
    }

    fn decode_request(&self, body: serde_json::Value) -> anyhow::Result<Request> {
        crate::translation::openai_decode::decode_openai_request(body)
    }

    fn encode_request(&self, request: &Request) -> anyhow::Result<serde_json::Value> {
        let mut body = serde_json::json!({
            "model": request.llm_request.model.as_deref().unwrap_or(""),
            "messages": encode_messages(&request.llm_request.messages),
        });

        let system = encode_system(&request.llm_request.instructions);
        if !system.is_null() {
            body["system"] = system;
        }

        if let Some(sampling) = &request.llm_request.sampling {
            if let Some(t) = sampling.temperature {
                body["temperature"] = serde_json::json!(t);
            }
            if let Some(t) = sampling.top_p {
                body["top_p"] = serde_json::json!(t);
            }
            if let Some(t) = sampling.top_k {
                body["top_k"] = serde_json::json!(t);
            }
        }

        if let Some(output) = &request.llm_request.output {
            if let Some(max) = output.max_output_tokens {
                body["max_tokens"] = serde_json::json!(max);
            }
            if let Some(stop) = &output.stop {
                body["stop_sequences"] = serde_json::json!(stop);
            }
        }

        if !request.llm_request.tools.is_empty() {
            body["tools"] = serde_json::json!(&request
                .llm_request
                .tools
                .iter()
                .map(encode_tool_definition)
                .collect::<Vec<_>>());
            if let Some(tc) = &request.llm_request.tool_choice {
                body["tool_choice"] = encode_tool_choice(tc);
            }
        }

        if request.llm_request.stream {
            body["stream"] = serde_json::json!(true);
        }

        for (k, v) in &request.extra_body {
            body[k] = v.clone();
        }

        Ok(body)
    }

    fn decode_response(
        &self,
        _status: reqwest::StatusCode,
        body: serde_json::Value,
    ) -> anyhow::Result<LlmResponse> {
        let content = body
            .get("content")
            .and_then(|c| c.as_array())
            .map(|arr| arr.iter().filter_map(decode_content_block).collect())
            .unwrap_or_default();

        let tool_calls = body.get("content").and_then(|c| c.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|b| {
                    if b.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        Some(crate::protocol::ToolCall {
                            id: b
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            function: crate::protocol::ToolCallFunction {
                                name: b
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                arguments: b
                                    .get("input")
                                    .map(|v| v.to_string())
                                    .unwrap_or_default(),
                            },
                        })
                    } else {
                        None
                    }
                })
                .collect()
        });

        let usage = body.get("usage").map(decode_usage).unwrap_or_default();
        let stop_reason = body
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(decode_stop_reason)
            .unwrap_or(crate::protocol::StopReason::Unknown);

        Ok(LlmResponse::Aggregate(Box::new(
            crate::protocol::AggLlmResponse {
                id: body
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                content,
                tool_calls: tool_calls.filter(|v: &Vec<_>| !v.is_empty()),
                stop_reason,
                usage,
                model: body
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                extra: Default::default(),
                preservation: Default::default(),
            },
        )))
    }

    fn decode_sse_chunk(
        &self,
        chunk: &str,
    ) -> anyhow::Result<Option<crate::protocol::LlmResponseStreamEvent>> {
        if chunk.is_empty() || chunk.starts_with(":") {
            return Ok(None);
        }
        let json_str = chunk.strip_prefix("data: ").unwrap_or(chunk);
        let event: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("invalid sse json: {}", e))?;
        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

        let delta = match event_type {
            "content_block_delta" => event
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| crate::protocol::LlmResponseChunk::ContentDelta {
                    block: crate::protocol::ContentBlock::Text {
                        text: s.to_string(),
                    },
                }),
            "message_stop" => Some(crate::protocol::LlmResponseChunk::Done),
            _ => None,
        };

        if let Some(delta) = delta {
            Ok(Some(
                crate::protocol::LlmResponseStreamEvent::new(delta).with_provider_event(event),
            ))
        } else {
            Ok(None)
        }
    }
}

fn encode_system(instructions: &[crate::protocol::InstructionBlock]) -> serde_json::Value {
    let texts: Vec<String> = instructions
        .iter()
        .flat_map(|i| {
            i.content.iter().filter_map(|b| match b {
                crate::protocol::ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
        })
        .collect();
    if texts.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::json!(texts.join("\n"))
    }
}

fn encode_messages(messages: &[crate::protocol::Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| {
            let mut obj = serde_json::json!({
                "role": encode_role(msg.role),
                "content": encode_content(&msg.content),
            });
            if let Some(name) = &msg.name {
                obj["name"] = serde_json::json!(name);
            }
            obj
        })
        .collect()
}

fn encode_role(role: crate::protocol::Role) -> &'static str {
    match role {
        crate::protocol::Role::System => "system",
        crate::protocol::Role::User => "user",
        crate::protocol::Role::Assistant => "assistant",
        crate::protocol::Role::Tool => "user",
    }
}

fn encode_content(blocks: &[crate::protocol::ContentBlock]) -> serde_json::Value {
    if blocks.is_empty() {
        return serde_json::json!("");
    }
    if blocks.len() == 1 {
        match &blocks[0] {
            crate::protocol::ContentBlock::Text { text } => return serde_json::json!(text),
            crate::protocol::ContentBlock::Image { source, mime_type } => {
                return serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": mime_type,
                        "data": match source {
                            crate::protocol::ImageSource::Base64 { data } => data.clone(),
                            crate::protocol::ImageSource::Url { url } => url.clone(),
                        }
                    }
                });
            }
            crate::protocol::ContentBlock::ToolResult { call_id, output } => {
                return serde_json::json!({
                    "type": "tool_result",
                    "tool_call_id": call_id,
                    "content": output,
                });
            }
        }
    }
    serde_json::json!(blocks
        .iter()
        .map(|b| match b {
            crate::protocol::ContentBlock::Text { text } => {
                serde_json::json!({ "type": "text", "text": text })
            }
            crate::protocol::ContentBlock::Image { source, mime_type } => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": mime_type,
                    "data": match source {
                        crate::protocol::ImageSource::Base64 { data } => data.clone(),
                        crate::protocol::ImageSource::Url { url } => url.clone(),
                    }
                }
            }),
            crate::protocol::ContentBlock::ToolResult { call_id, output } => serde_json::json!({
                "type": "tool_result",
                "tool_call_id": call_id,
                "content": output,
            }),
        })
        .collect::<Vec<_>>())
}

fn encode_tool_choice(tc: &crate::protocol::ToolChoice) -> serde_json::Value {
    match tc {
        crate::protocol::ToolChoice::None => serde_json::json!("none"),
        crate::protocol::ToolChoice::Auto => serde_json::json!("auto"),
        crate::protocol::ToolChoice::Required => serde_json::json!({"type": "any"}),
        crate::protocol::ToolChoice::Named { name } => {
            serde_json::json!({"type": "tool", "name": name})
        }
    }
}

fn encode_tool_definition(tool: &crate::protocol::ToolDefinition) -> serde_json::Value {
    let mut out = serde_json::json!({
        "name": tool.name,
        "input_schema": tool.parameters,
    });
    if let Some(desc) = &tool.description {
        out["description"] = serde_json::json!(desc);
    }
    out
}

fn decode_content_block(b: &serde_json::Value) -> Option<crate::protocol::ContentBlock> {
    match b.get("type").and_then(|t| t.as_str()) {
        Some("text") => {
            b.get("text")
                .and_then(|v| v.as_str())
                .map(|s| crate::protocol::ContentBlock::Text {
                    text: s.to_string(),
                })
        }
        _ => None,
    }
}

fn decode_usage(u: &serde_json::Value) -> Usage {
    Usage {
        prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        total_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
            + u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        cached_tokens: u
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64()),
        cache_creation_tokens: u.get("cache_read_input_tokens").and_then(|v| v.as_u64()),
        reasoning_tokens: None,
    }
}

fn decode_stop_reason(s: &str) -> StopReason {
    match s {
        "end_turn" => StopReason::Stop,
        "max_tokens" => StopReason::Length,
        "tool_use" => StopReason::ToolCalls,
        "stop_sequence" => StopReason::Stop,
        _ => StopReason::Unknown,
    }
}
