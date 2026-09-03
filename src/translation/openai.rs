use crate::protocol::{
    ContentBlock, LlmResponse, LlmResponseStreamEvent, Message, Request, Usage, WireFormat,
};
use crate::translation::openai_decode::decode_openai_request;
use crate::translation::WireCodec;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct OpenAiChatCodec;

impl WireCodec for OpenAiChatCodec {
    fn wire_format(&self) -> WireFormat {
        WireFormat::OpenAiChat
    }

    fn decode_request(&self, body: Value) -> anyhow::Result<Request> {
        decode_openai_request(body)
    }

    fn encode_request(&self, request: &Request) -> anyhow::Result<Value> {
        let mut body = serde_json::json!({
            "model": request.llm_request.model.as_deref().unwrap_or(""),
            "messages": encode_messages(&request.llm_request.messages, &request.llm_request.instructions),
            "stream": request.llm_request.stream,
        });

        if let Some(sampling) = &request.llm_request.sampling {
            if let Some(t) = sampling.temperature {
                body["temperature"] = serde_json::json!(t);
            }
            if let Some(t) = sampling.top_p {
                body["top_p"] = serde_json::json!(t);
            }
            if let Some(seed) = sampling.seed {
                body["seed"] = serde_json::json!(seed);
            }
        }

        if let Some(output) = &request.llm_request.output {
            if let Some(max) = output.max_output_tokens {
                body["max_tokens"] = serde_json::json!(max);
            }
            if let Some(stop) = &output.stop {
                body["stop"] = serde_json::json!(stop);
            }
        }

        if !request.llm_request.tools.is_empty() {
            body["tools"] = serde_json::json!(request
                .llm_request
                .tools
                .iter()
                .map(encode_tool)
                .collect::<Vec<_>>());
            if let Some(tc) = &request.llm_request.tool_choice {
                body["tool_choice"] = encode_tool_choice(tc);
            }
        }

        Ok(body)
    }

    fn decode_response(
        &self,
        _status: reqwest::StatusCode,
        body: serde_json::Value,
    ) -> anyhow::Result<LlmResponse> {
        let choices = body
            .get("choices")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let choice = choices.first().cloned().unwrap_or_else(
            || serde_json::json!({"message": {"role": "assistant", "content": null}}),
        );
        let message = choice
            .get("message")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({"role": "assistant", "content": null}));
        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| {
                vec![ContentBlock::Text {
                    text: s.to_string(),
                }]
            })
            .unwrap_or_default();
        let tool_calls = message
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|tc| crate::protocol::ToolCall {
                        id: tc
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        function: crate::protocol::ToolCallFunction {
                            name: tc
                                .get("function")
                                .and_then(|f| f.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            arguments: tc
                                .get("function")
                                .and_then(|f| f.get("arguments"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        },
                    })
                    .collect()
            });
        let usage = body.get("usage").map(decode_usage).unwrap_or_default();
        let stop_reason = choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default();

        Ok(LlmResponse::Aggregate(Box::new(
            crate::protocol::AggLlmResponse {
                id: body
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                content,
                tool_calls,
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

    fn decode_sse_chunk(&self, chunk: &str) -> anyhow::Result<Option<LlmResponseStreamEvent>> {
        if chunk.is_empty() || chunk.starts_with(":") {
            return Ok(None);
        }
        let json_str = chunk.strip_prefix("data: ").unwrap_or(chunk);
        if json_str.trim() == "[DONE]" {
            return Ok(Some(LlmResponseStreamEvent::new(
                crate::protocol::LlmResponseChunk::Done,
            )));
        }
        let event: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("invalid sse json: {}", e))?;
        let delta = event
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("delta"))
            .cloned()
            .unwrap_or_default();

        let content = delta
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| ContentBlock::Text {
                text: s.to_string(),
            });
        let tool_calls = delta
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|tc| crate::protocol::ToolCall {
                        id: tc
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        function: crate::protocol::ToolCallFunction {
                            name: tc
                                .get("function")
                                .and_then(|f| f.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            arguments: tc
                                .get("function")
                                .and_then(|f| f.get("arguments"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        },
                    })
                    .collect()
            });

        let chunk = if let Some(tool) = tool_calls {
            crate::protocol::LlmResponseChunk::ToolCallDelta { calls: tool }
        } else if let Some(block) = content {
            crate::protocol::LlmResponseChunk::ContentDelta { block }
        } else {
            crate::protocol::LlmResponseChunk::Progress
        };

        let usage = event.get("usage").map(decode_usage);
        let mut ev = LlmResponseStreamEvent::new(chunk).with_provider_event(event);
        if let Some(u) = usage {
            ev.usage = Some(u);
        }
        Ok(Some(ev))
    }
}

fn encode_messages(
    messages: &[Message],
    instructions: &[crate::protocol::InstructionBlock],
) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for inst in instructions {
        out.push(serde_json::json!({
            "role": inst.role.to_wire(),
            "content": encode_content(&inst.content),
        }));
    }
    for msg in messages {
        let mut obj = serde_json::json!({
            "role": msg.role.to_wire(),
            "content": encode_content(&msg.content),
        });
        if let Some(tools) = &msg.tool_calls {
            obj["tool_calls"] =
                serde_json::json!(tools.iter().map(encode_tool_call).collect::<Vec<_>>());
        }
        if let Some(ref tool_call_id) = msg.tool_call_id {
            obj["tool_call_id"] = serde_json::json!(tool_call_id);
        }
        if let Some(ref name) = msg.name {
            obj["name"] = serde_json::json!(name);
        }
        out.push(obj);
    }
    out
}

fn encode_tool(tool: &crate::protocol::ToolDefinition) -> serde_json::Value {
    let mut function = serde_json::json!({
        "name": tool.name,
        "parameters": tool.parameters,
    });
    if let Some(desc) = &tool.description {
        function["description"] = serde_json::json!(desc);
    }
    let mut out = serde_json::json!({ "type": "function", "function": function });
    if let Some(strict) = tool.strict {
        out["strict"] = serde_json::json!(strict);
    }
    out
}

fn encode_tool_call(tc: &crate::protocol::ToolCall) -> serde_json::Value {
    serde_json::json!({
        "id": tc.id,
        "type": "function",
        "function": {
            "name": tc.function.name,
            "arguments": tc.function.arguments,
        },
    })
}

fn encode_content(blocks: &[ContentBlock]) -> serde_json::Value {
    if blocks.is_empty() {
        return serde_json::Value::Null;
    }
    if blocks.len() == 1 {
        if let ContentBlock::Text { text } = &blocks[0] {
            return serde_json::json!(text);
        }
    }
    serde_json::json!(blocks
        .iter()
        .map(|b| match b {
            ContentBlock::Text { text } => serde_json::json!({
                "type": "text",
                "text": text,
            }),
            ContentBlock::Image { source, .. } => serde_json::json!({
                "type": "image_url",
                "image_url": { "url": source.to_url() },
            }),
            ContentBlock::ToolResult { call_id, output } => {
                serde_json::json!({
                    "type": "tool_result",
                    "tool_call_id": call_id,
                    "output": output,
                })
            }
        })
        .collect::<Vec<_>>())
}

fn encode_tool_choice(tc: &crate::protocol::ToolChoice) -> serde_json::Value {
    match tc {
        crate::protocol::ToolChoice::None => serde_json::json!("none"),
        crate::protocol::ToolChoice::Auto => serde_json::json!("auto"),
        crate::protocol::ToolChoice::Required => {
            serde_json::json!({ "type": "function", "function": { "name": "" } })
        }
        crate::protocol::ToolChoice::Named { name } => {
            serde_json::json!({ "type": "function", "function": { "name": name } })
        }
    }
}

fn decode_usage(u: &serde_json::Value) -> Usage {
    Usage {
        prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        completion_tokens: u
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        cached_tokens: u
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64()),
        cache_creation_tokens: u
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cache_creation_tokens"))
            .and_then(|v| v.as_u64()),
        reasoning_tokens: u
            .get("completion_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|v| v.as_u64()),
    }
}
