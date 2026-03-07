use rexos_llm::openai_compat::{ToolCall, ToolFunction};

#[derive(Debug, serde::Deserialize)]
struct JsonToolCall {
    #[serde(alias = "function_name")]
    name: String,
    #[serde(alias = "args")]
    #[serde(default)]
    arguments: Option<serde_json::Value>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

pub(crate) fn normalize_tool_arguments(tool_name: &str, raw_arguments_json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw_arguments_json) else {
        return raw_arguments_json.to_string();
    };

    let Some(obj) = v.as_object() else {
        return raw_arguments_json.to_string();
    };

    let matches_name = obj
        .get("function")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("name").and_then(|v| v.as_str()))
        .or_else(|| obj.get("function_name").and_then(|v| v.as_str()))
        .map(|name| name == tool_name)
        .unwrap_or(true);
    if !matches_name {
        return raw_arguments_json.to_string();
    }

    let Some(inner) = obj.get("arguments") else {
        return raw_arguments_json.to_string();
    };

    if let Some(s) = inner.as_str() {
        return s.to_string();
    }

    serde_json::to_string(inner).unwrap_or_else(|_| raw_arguments_json.to_string())
}

pub(crate) fn parse_tool_calls_from_json_content(content: &str) -> Option<Vec<ToolCall>> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(calls) = parse_json_tool_calls_from_value(value) {
            return Some(into_tool_calls(calls));
        }
    }

    let calls = extract_json_tool_calls_from_text(trimmed);
    if calls.is_empty() {
        return None;
    }
    Some(into_tool_calls(calls))
}

fn into_tool_calls(calls: Vec<JsonToolCall>) -> Vec<ToolCall> {
    let mut out = Vec::new();
    for (idx, call) in calls.into_iter().enumerate() {
        let args_value = call
            .arguments
            .unwrap_or_else(|| serde_json::Value::Object(call.extra));
        let args = if let Some(s) = args_value.as_str() {
            s.to_string()
        } else {
            serde_json::to_string(&args_value).unwrap_or_else(|_| "{}".to_string())
        };
        out.push(ToolCall {
            id: format!("call_json_{}", idx + 1),
            kind: "function".to_string(),
            function: ToolFunction {
                name: call.name,
                arguments: args,
            },
        });
    }
    out
}

pub(crate) fn truncate_tool_result_with_flag(output: String, max_chars: usize) -> (String, bool) {
    if max_chars == 0 {
        return (String::new(), !output.is_empty());
    }

    let total_chars = output.chars().count();
    if total_chars <= max_chars {
        return (output, false);
    }

    let head_chars = max_chars / 2;
    let tail_chars = max_chars - head_chars;
    let omitted = total_chars.saturating_sub(max_chars);

    let head: String = output.chars().take(head_chars).collect();
    let tail: String = output
        .chars()
        .skip(total_chars.saturating_sub(tail_chars))
        .collect();

    (
        format!("{head}\n\n[... omitted {omitted} chars ...]\n\n{tail}"),
        true,
    )
}

fn parse_json_tool_calls_from_value(value: serde_json::Value) -> Option<Vec<JsonToolCall>> {
    if let Some(arr) = value.as_array() {
        let mut calls = Vec::new();
        for item in arr {
            calls.push(serde_json::from_value::<JsonToolCall>(item.clone()).ok()?);
        }
        return Some(calls);
    }

    serde_json::from_value::<JsonToolCall>(value)
        .ok()
        .map(|c| vec![c])
}

fn extract_json_tool_calls_from_text(content: &str) -> Vec<JsonToolCall> {
    let mut calls = Vec::new();
    for (start, _) in content.match_indices('{') {
        if calls.len() >= 16 {
            break;
        }
        let Some(end) = find_balanced_json_object_end(content, start) else {
            continue;
        };
        let slice = &content[start..end];
        let Ok(value) = serde_json::from_str::<serde_json::Value>(slice) else {
            continue;
        };
        let Some(mut parsed) = parse_json_tool_calls_from_value(value) else {
            continue;
        };
        calls.append(&mut parsed);
    }
    calls
}

fn find_balanced_json_object_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if start >= bytes.len() || bytes[start] != b'{' {
        return None;
    }

    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if b == b'\\' {
                escape = true;
                continue;
            }
            if b == b'"' {
                in_string = false;
                continue;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_tool_arguments, parse_tool_calls_from_json_content,
        truncate_tool_result_with_flag,
    };

    #[test]
    fn parses_embedded_json_tool_calls_from_freeform_text() {
        let content = r#"Before text {"function_name":"fs_write","args":{"path":"hello.txt","content":"hi"}} after"#;
        let calls = parse_tool_calls_from_json_content(content).expect("expected parsed tool call");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "fs_write");
    }

    #[test]
    fn unwraps_nested_tool_arguments_payload() {
        let raw = r#"{"function_name":"fs_write","arguments":{"path":"a.txt"}}"#;
        assert_eq!(
            normalize_tool_arguments("fs_write", raw),
            r#"{"path":"a.txt"}"#
        );
    }

    #[test]
    fn truncation_preserves_omission_marker() {
        let (out, truncated) = truncate_tool_result_with_flag("abcdefghij".to_string(), 6);
        assert!(truncated);
        assert!(out.contains("omitted"), "{out}");
    }
}
