//! LLM client for TRDC
//!
//! Handles communication with local LLM (LM Studio) for semantic summarization.
//! Processes large outputs in chunks to avoid memory issues.

use crate::config::LlmConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const CHUNK_SIZE: usize = 2000;
const MAX_CHUNKS: usize = 100;

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: usize,
    temperature: f32,
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

#[derive(Debug, Clone)]
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

pub fn summarize(
    output: &str,
    user_cmd: &str,
    user_context: Option<&str>,
    config: &LlmConfig,
) -> Result<(OutputJson, TokenUsage)> {
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
        ));
    }

    let output_path = write_output_to_file(output, user_cmd)?;

    let chunks = chunk_output(output);
    let total = chunks.len();

    eprintln!("[trdc] Processing {total} chunks...");

    let mut accumulated_summary = String::new();
    let mut total_input_tokens = 0usize;
    let mut total_output_tokens = 0usize;

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_num = i + 1;
        let prompt = build_summarize_prompt(chunk_num, total, chunk, user_context);
        let result = call_llm(&prompt, config)?;

        if let Some(ref usage) = result.usage {
            total_input_tokens += usage.prompt_tokens;
            total_output_tokens += usage.completion_tokens;
        }

        let response = result.content;
        let clean_response = response.trim().to_string();

        if accumulated_summary.is_empty() {
            accumulated_summary = clean_response;
        } else {
            accumulated_summary = format!("{accumulated_summary}\n\n{clean_response}");
        }
    }

    Ok((
        OutputJson {
            summary: accumulated_summary,
            complete_output_path: output_path.to_string_lossy().to_string(),
        },
        TokenUsage {
            input_tokens: total_input_tokens,
            output_tokens: total_output_tokens,
        },
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

pub fn chunk_output(output: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let bytes = output.as_bytes();

    if bytes.len() <= CHUNK_SIZE {
        return vec![output.to_string()];
    }

    let mut start = 0;
    while start < bytes.len() && chunks.len() < MAX_CHUNKS {
        let end = std::cmp::min(start + CHUNK_SIZE, bytes.len());

        if end < bytes.len() {
            if let Some(nl) = bytes[start..end].iter().rposition(|&b| b == b'\n') {
                chunks.push(String::from_utf8_lossy(&bytes[start..start + nl]).to_string());
                start += nl + 1;
            } else {
                chunks.push(String::from_utf8_lossy(&bytes[start..end]).to_string());
                start = end;
            }
        } else {
            chunks.push(String::from_utf8_lossy(&bytes[start..end]).to_string());
            break;
        }
    }

    if start < bytes.len() && chunks.len() == MAX_CHUNKS {
        chunks.push(String::from_utf8_lossy(&bytes[start..]).to_string());
    }

    chunks
}

fn call_llm(prompt: &str, config: &LlmConfig) -> Result<LlmResult> {
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
    };

    let timeout = Duration::from_secs(config.timeout_secs);
    let start = Instant::now();

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into();

    let mut response = agent
        .post(&config.endpoint_url)
        .send_json(&request)
        .map_err(|e| match &e {
            ureq::Error::StatusCode(code) => {
                anyhow::anyhow!("LLM API returned status {code}")
            }
            _ => anyhow::anyhow!("LLM request failed: {e}"),
        })?;

    let elapsed = start.elapsed().as_millis();

    let body: String = response
        .body_mut()
        .read_to_string()
        .context("Failed to read LLM response body")?;
    let chat_resp: ChatResponse =
        serde_json::from_str(&body).context("Failed to parse LLM response")?;

    eprintln!("[trdc] LLM response in {elapsed}ms");

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

#[allow(dead_code)]
fn build_json_input(user_cmd: &str, output: &str, context: Option<&str>) -> String {
    let escaped_output = output
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");

    let escaped_cmd = user_cmd.replace('\\', "\\\\").replace('"', "\\\"");

    let escaped_context = context
        .map(|c| c.replace('\\', "\\\\").replace('"', "\\\""))
        .unwrap_or_default();

    format!(
        r#"{{
  "user_cmd": "{escaped_cmd}",
  "cmd_output": "{escaped_output}",
  "context_provided": "{escaped_context}"
}}"#
    )
}

pub fn build_output(summary: &str, output_path: &str) -> String {
    format!("Summary:\n{summary}\n\nFull output saved to: {output_path}")
}

fn build_summarize_prompt(
    chunk_num: usize,
    total_chunks: usize,
    chunk: &str,
    context: Option<&str>,
) -> String {
    let context_part = match context {
        Some(ctx) if !ctx.is_empty() => format!("Context: {ctx}\n"),
        _ => String::new(),
    };

    if chunk_num == 1 {
        format!(
            r"Analyze this output chunk 1/{total_chunks} and provide a brief summary.
{context_part}
CHUNK:
{chunk}

Provide a concise summary in plain text (not JSON)."
        )
    } else {
        format!(
            r"Continue analysis of output chunk {chunk_num}/{total_chunks}:
{context_part}
CHUNK:
{chunk}

Append to previous summary. Output plain text summary."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> LlmConfig {
        LlmConfig {
            endpoint_url: "http://localhost:1234/v1/chat/completions".to_string(),
            model_name: "local-model".to_string(),
            timeout_secs: 5,
            max_output_tokens: 200,
            max_input_chars: 4000,
            temperature: 0.7,
        }
    }

    #[test]
    fn test_build_json_input() {
        let input = build_json_input("echo hello", "hello world", Some("test context"));
        assert!(input.contains("echo hello"));
        assert!(input.contains("hello world"));
        assert!(input.contains("test context"));
    }

    #[test]
    fn test_build_json_input_no_context() {
        let input = build_json_input("echo hello", "hello world", None);
        assert!(input.contains("echo hello"));
        assert!(input.contains("\"\""));
    }

    #[test]
    fn test_chunk_output_small() {
        let output = "short output";
        let chunks = chunk_output(output);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "short output");
    }

    #[test]
    fn test_chunk_output_large() {
        let output = "x".repeat(5000);
        let chunks = chunk_output(&output);
        assert!(chunks.len() > 1);
        assert!(chunks.len() <= 3);
    }

    #[test]
    fn test_build_output() {
        let output = build_output("test summary", "/path/to/file.txt");
        assert!(output.contains("test summary"));
        assert!(output.contains("/path/to/file.txt"));
        assert!(output.contains("Summary:"));
    }
}
