use crate::protocol::{
    ContentBlock, ImageSource, InstructionBlock, LlmRequest, Message, OutputParams, Request, Role,
    SamplingParams, ToolChoice, ToolDefinition,
};
use serde_json::Value;

pub fn decode_openai_request(body: Value) -> anyhow::Result<Request> {
    let model = body.get("model").and_then(|v| v.as_str()).map(String::from);
    let messages: Vec<Message> = body
        .get("messages")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(decode_message).collect())
        .unwrap_or_default();

    let mut instructions: Vec<InstructionBlock> = Vec::new();
    let mut conversation_messages: Vec<Message> = Vec::new();
    for msg in messages {
        if msg.role == Role::System {
            instructions.push(InstructionBlock {
                role: msg.role,
                content: msg.content,
            });
        } else {
            conversation_messages.push(msg);
        }
    }

    let tools = body
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(decode_tool).collect())
        .unwrap_or_default();
    let tool_choice = body.get("tool_choice").and_then(decode_tool_choice);

    let sampling = SamplingParams {
        temperature: body
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32),
        top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|f| f as f32),
        top_k: None,
        seed: body.get("seed").and_then(|v| v.as_i64()),
    };
    let output = OutputParams {
        max_output_tokens: body
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|u| u as u32),
        stop: body.get("stop").and_then(|v| {
            if let Some(arr) = v.as_array() {
                Some(
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect(),
                )
            } else {
                v.as_str().map(|s| vec![s.to_string()])
            }
        }),
    };
    let stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut extra_body = std::collections::HashMap::new();
    let known = [
        "model",
        "messages",
        "tools",
        "tool_choice",
        "temperature",
        "top_p",
        "seed",
        "max_tokens",
        "stop",
        "stream",
        "n",
        "presence_penalty",
        "frequency_penalty",
        "logit_bias",
        "user",
        "response_format",
    ];
    for (k, v) in body.as_object().iter().flat_map(|o| o.iter()) {
        if !known.contains(&k.as_str()) {
            extra_body.insert(k.clone(), v.clone());
        }
    }

    Ok(Request {
        llm_request: LlmRequest {
            model,
            instructions,
            messages: conversation_messages,
            tools,
            tool_choice,
            sampling: Some(sampling),
            output: Some(output),
            stream,
            extra: Default::default(),
        },
        metadata: None,
        extra_body,
    })
}

fn decode_message(v: &Value) -> Option<Message> {
    let role: Role = v.get("role").and_then(|r| r.as_str())?.parse().ok()?;
    let content = v.get("content").map(decode_content).unwrap_or_default();
    let tool_calls = v.get("tool_calls").and_then(|t| t.as_array()).map(|arr| {
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
    let tool_call_id = v
        .get("tool_call_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let name = v.get("name").and_then(|v| v.as_str()).map(String::from);

    Some(Message {
        role,
        content,
        name,
        tool_calls,
        tool_call_id,
    })
}

fn decode_content(v: &Value) -> Vec<ContentBlock> {
    if let Some(text) = v.as_str() {
        return vec![ContentBlock::Text {
            text: text.to_string(),
        }];
    }
    if let Some(arr) = v.as_array() {
        return arr
            .iter()
            .filter_map(|b| {
                let t = b.get("type").and_then(|t| t.as_str())?;
                match t {
                    "text" => b
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(|s| ContentBlock::Text {
                            text: s.to_string(),
                        }),
                    "image_url" => {
                        let url = b.get("image_url")?.get("url")?.as_str()?;
                        Some(ContentBlock::Image {
                            source: ImageSource::Url {
                                url: url.to_string(),
                            },
                            mime_type: "image/*".to_string(),
                        })
                    }
                    _ => None,
                }
            })
            .collect();
    }
    Vec::new()
}

fn decode_tool(v: &Value) -> Option<ToolDefinition> {
    let function = v.get("function")?;
    Some(ToolDefinition {
        name: function.get("name")?.as_str()?.to_string(),
        description: function
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
        parameters: function.get("parameters").cloned().unwrap_or(Value::Null),
        strict: v.get("strict").and_then(|v| v.as_bool()),
    })
}

fn decode_tool_choice(v: &Value) -> Option<ToolChoice> {
    if let Some(s) = v.as_str() {
        return match s {
            "none" => Some(ToolChoice::None),
            "auto" => Some(ToolChoice::Auto),
            "required" => Some(ToolChoice::Required),
            _ => None,
        };
    }
    if let Some(obj) = v.as_object() {
        if obj.get("type").and_then(|t| t.as_str()) == Some("function") {
            if let Some(name) = obj
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
            {
                return Some(ToolChoice::Named {
                    name: name.to_string(),
                });
            }
        }
    }
    None
}
