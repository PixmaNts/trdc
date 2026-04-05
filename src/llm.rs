//! LLM client for TRDC
//!
//! Handles communication with local LLM (LM Studio) for semantic summarization.
//! Uses single-request summarization with smart truncation for large outputs.

use crate::config::LlmConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: usize,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code, clippy::struct_field_names)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    #[allow(dead_code)]
    pub total_tokens: usize,
}

// Streaming response structs for SSE format
#[derive(Debug, Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LlmResult {
    pub content: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputJson {
    pub summary: String,
    pub complete_output_path: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TokenUsage {
    #[allow(dead_code)]
    pub input_tokens: usize,
    #[allow(dead_code)]
    pub output_tokens: usize,
}

/// Smart truncation that keeps head + tail of output when it exceeds max_chars.
/// Uses 20% for head, 80% for tail to preserve error messages at the end.
fn smart_truncate(input: &str, max_chars: usize) -> String {
    let input_len = input.chars().count();
    if input_len <= max_chars {
        return input.to_string();
    }

    let head_chars = (max_chars * 20) / 100; // 20% for head
    let tail_chars = max_chars - head_chars; // 80% for tail

    let chars: Vec<char> = input.chars().collect();
    let tail_start = input_len.saturating_sub(tail_chars);

    let head: String = chars[..head_chars].iter().collect();
    let tail: String = chars[tail_start..].iter().collect();

    // Count newlines between head and tail in the ORIGINAL string
    // This correctly handles cases where head/tail split lines
    let removed_lines = if head_chars < tail_start {
        chars[head_chars..tail_start].iter().filter(|&&c| c == '\n').count()
    } else {
        0
    };

    if removed_lines > 0 {
        format!("{}\n\n[... {} lines truncated ...]\n\n{}", head, removed_lines, tail)
    } else {
        format!("{}{}", head, tail)
    }
}

/// Builds a single prompt for the LLM with the (potentially truncated) output.
fn build_single_prompt(output: &str, user_cmd: &str, context: Option<&str>) -> String {
    let context_part = match context {
        Some(ctx) if !ctx.is_empty() => format!("Context: {}\n", ctx),
        _ => String::new(),
    };

    format!(
        r"Analyze the output of this command: {}

{context_part}OUTPUT:
{}

Provide a concise summary in plain text.",
        user_cmd, output
    )
}

pub async fn summarize(
    output: &str,
    user_cmd: &str,
    user_context: Option<&str>,
    config: &LlmConfig,
) -> Result<(OutputJson, TokenUsage, bool)> {
    if output.trim().is_empty() {
        return Ok((
            OutputJson {
                summary: "No output".to_string(),
                complete_output_path: String::new(),
            },
            TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
            false,
        ));
    }

    let output_path = write_output_to_file(output, user_cmd)?;

    // Apply smart truncation if output exceeds max_input_chars
    let truncated_output = smart_truncate(output, config.max_input_chars);
    let was_truncated = truncated_output != output;

    if was_truncated && std::io::stderr().is_terminal() {
        eprintln!(
            "[trdc] Output truncated from {} to {} chars",
            output.chars().count(),
            truncated_output.chars().count()
        );
    }

    let prompt = build_single_prompt(&truncated_output, user_cmd, user_context);

    println!("Summary:");
    std::io::stdout().flush().ok();

    let streamed_content = call_llm_streaming(&prompt, config, |token| {
        print!("{token}");
        std::io::stdout().flush().ok();
    })
    .await?;

    if !streamed_content.ends_with('\n') {
        println!();
    }

    let clean_response = streamed_content.trim().to_string();
    let response_chars = clean_response.chars().count();

    // For streaming, we don't get usage info - estimate based on chars/4
    let output_tokens = response_chars.div_ceil(4);
    let input_tokens = prompt.chars().count().div_ceil(4);

    Ok((
        OutputJson {
            summary: clean_response,
            complete_output_path: output_path.to_string_lossy().to_string(),
        },
        TokenUsage {
            input_tokens,
            output_tokens,
        },
        true, // streaming happened
    ))
}

pub fn write_output_to_file(output: &str, user_cmd: &str) -> Result<PathBuf> {
    let sanitized = user_cmd
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("trdc_output_{sanitized}_{timestamp}.txt");

    let output_dir = PathBuf::from("/tmp/trdc");

    fs::create_dir_all(&output_dir)?;
    let path = output_dir.join(&filename);
    fs::write(&path, output)?;

    Ok(path)
}

#[allow(dead_code)]
async fn call_llm(prompt: &str, config: &LlmConfig) -> Result<LlmResult> {
    let request = ChatRequest {
        model: &config.model_name,
        messages: vec![
            Message {
                role: "system",
                content: "You are a CLI output analyzer. Provide concise, well-formatted summaries in plain text.",
            },
            Message {
                role: "user",
                content: prompt,
            },
        ],
        max_tokens: config.max_output_tokens,
        temperature: config.temperature,
        stream: None,
    };

    let timeout = Duration::from_secs(config.timeout_secs);

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .post(&config.endpoint_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!("LLM request timed out")
            } else if e.is_status() {
                let status = e
                    .status()
                    .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                anyhow::anyhow!("LLM API returned status {status}")
            } else {
                anyhow::anyhow!("LLM request failed: {e}")
            }
        })?;

    let chat_resp: ChatResponse = response
        .json()
        .await
        .context("Failed to parse LLM response")?;

    let content = chat_resp
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .context("LLM returned no choices")?;

    Ok(LlmResult {
        content,
        usage: chat_resp.usage,
    })
}

/// Streams LLM response token-by-token to stdout
/// Returns the full accumulated response text
async fn call_llm_streaming(
    prompt: &str,
    config: &LlmConfig,
    mut on_token: impl FnMut(&str),
) -> Result<String> {
    let request = ChatRequest {
        model: &config.model_name,
        messages: vec![
            Message {
                role: "system",
                content: "You are a CLI output analyzer. Provide concise, well-formatted summaries in plain text.",
            },
            Message {
                role: "user",
                content: prompt,
            },
        ],
        max_tokens: config.max_output_tokens,
        temperature: config.temperature,
        stream: Some(true),
    };

    let timeout = Duration::from_secs(config.timeout_secs);

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .post(&config.endpoint_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!("LLM request timed out")
            } else if e.is_status() {
                let status = e
                    .status()
                    .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                anyhow::anyhow!("LLM API returned status {status}")
            } else {
                anyhow::anyhow!("LLM request failed: {e}")
            }
        })?;

    // Use bytes_stream() to get async stream of bytes
    let mut stream = response.bytes_stream();

    let mut full_response = String::new();

    use futures_util::StreamExt;

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                // Parse SSE lines from the chunk
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // SSE format: "data: {...}"
                    if !line.starts_with("data:") {
                        continue;
                    }

                    let data = line.strip_prefix("data:").unwrap().trim();

                    // Stream terminator
                    if data == "[DONE]" {
                        break;
                    }

                    // Parse the JSON payload
                    match serde_json::from_str::<StreamResponse>(data) {
                        Ok(stream_resp) => {
                            for choice in stream_resp.choices {
                                if let Some(content) = choice.delta.content {
                                    // Some providers send empty deltas
                                    if !content.is_empty() {
                                        on_token(&content);
                                        full_response.push_str(&content);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Malformed JSON - skip this line, log to stderr
                            eprintln!("[trdc] Warning: malformed SSE data: {}", data);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[trdc] Stream error: {}", e);
                break;
            }
        }
    }

    Ok(full_response)
}

pub fn build_output(summary: &str, output_path: &str, skip_summary: bool) -> String {
    if skip_summary {
        format!("\nFull output saved to: {output_path}")
    } else {
        format!("Summary:\n{summary}\n\nFull output saved to: {output_path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_truncate_fits() {
        let input = "short output";
        let result = smart_truncate(input, 100);
        assert_eq!(result, "short output");
    }

    #[test]
    fn test_smart_truncate_truncates() {
        let input = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10";
        let result = smart_truncate(input, 30);
        assert!(result.contains("line1"));
        assert!(result.contains("line10"));
        // With head_chars=6 (20% of 30), tail_chars=24 (80% of 30)
        // removed_lines = 5 (lines 2-6)
        assert!(result.contains("[... 5 lines truncated ...]"));
    }

    #[test]
    fn test_smart_truncate_preserves_head_tail() {
        // With max_chars=25: head_chars=5, tail_chars=20
        // Head gets first 5 chars "aaaaa", tail gets last 20 chars
        let input = "aaaaa\nbbbbbbbbbb\ncccccccccc\ndddddddddd\neeeeeeeeee\nffffffffff";
        let result = smart_truncate(input, 25);
        // Head should be first 5 chars (20% of 25)
        assert!(result.contains("aaaaa"));
        // Tail should contain end of input
        assert!(result.contains("ffffffffff"));
        // Should have truncation marker (4 lines removed: bb, cc, dd, ee lines)
        assert!(result.contains("[... 4 lines truncated ...]"));
    }

    #[test]
    fn test_build_single_prompt() {
        let output = "test output";
        let user_cmd = "echo hello";
        let result = build_single_prompt(output, user_cmd, None);
        assert!(result.contains("echo hello"));
        assert!(result.contains("test output"));
        assert!(result.contains("OUTPUT:"));
    }

    #[test]
    fn test_build_single_prompt_with_context() {
        let output = "test output";
        let user_cmd = "echo hello";
        let context = "focus on errors";
        let result = build_single_prompt(output, user_cmd, Some(context));
        assert!(result.contains("Context: focus on errors"));
        assert!(result.contains("echo hello"));
        assert!(result.contains("test output"));
    }

    #[test]
    fn test_build_output() {
        let output = build_output("test summary", "/path/to/file.txt", false);
        assert!(output.contains("test summary"));
        assert!(output.contains("/path/to/file.txt"));
        assert!(output.contains("Summary:"));
    }

    #[test]
    fn test_build_output_skip_summary() {
        let output = build_output("test summary", "/path/to/file.txt", true);
        assert!(!output.contains("test summary"));
        assert!(output.contains("/path/to/file.txt"));
        assert!(!output.contains("Summary:"));
        assert!(output.contains("Full output saved to:"));
    }

    #[test]
    fn test_stream_response_parsing() {
        // Test that streaming response structs deserialize correctly
        let json = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some("hello"));
    }

    #[test]
    fn test_stream_response_multiple_choices() {
        let json = r#"{"choices":[{"delta":{"content":"hello"}},{"delta":{"content":"world"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
    }

    #[test]
    fn test_stream_response_empty_content() {
        let json = r#"{"choices":[{"delta":{"content":""}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some(""));
    }

    #[test]
    fn test_stream_response_missing_content() {
        let json = r#"{"choices":[{"delta":{}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }
}