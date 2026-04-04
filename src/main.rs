//! TRDC - Token Reducer CLI
//!
//! LLM-first context-aware command output summarization.

mod cli;
mod config;
mod executor;
mod filter;
mod history;
mod llm;
mod tracking;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use config::Config;
use history::{display_history, load};
use std::io::{IsTerminal, Read, Write};
use tracking::{SessionStats, TokenUsage, display_gain, estimate_tokens};

#[tokio::main]
async fn main() {
    match run().await {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

async fn run() -> Result<i32> {
    let cli = Cli::parse();

    if cli.show_history {
        let limit = if cli.full { None } else { Some(10) };
        let entries = load(limit)?;
        display_history(&entries, cli.full);
        return Ok(0);
    }

    if cli.verbose {
        eprintln!("[trdc] Loading configuration...");
    }

    let mut config = Config::load()?;

    if cli.verbose {
        eprintln!(
            "[trdc] Config loaded: {} tokens max output",
            config.llm.max_output_tokens
        );
    }

    if let Some(ref model) = cli.model {
        #[allow(clippy::redundant_clone)]
        let model_name = model.clone();
        config.llm.model_name = model_name;
        if cli.verbose {
            eprintln!("[trdc] Using model: {model}");
        }
    }

    if let Some(tokens) = cli.max_tokens {
        config.llm.max_output_tokens = tokens;
        if cli.verbose {
            eprintln!("[trdc] Max tokens: {tokens}");
        }
    }

    // Stdin detection (before config loading to allow stdin data to be used immediately)
    let mut stdin_data: Option<String> = if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        match std::io::stdin().read_to_string(&mut buf) {
            Ok(_) => match validate_stdin_data(buf) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("[trdc] Error: {e}");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("[trdc] Warning: Failed to read stdin: {e}");
                None
            }
        }
    } else {
        None
    };

    let user_cmd = if let Some(ref cmd) = cli.command {
        if cli.args.is_empty() {
            cmd.clone()
        } else {
            format!("{} {}", cmd, cli.args.join(" "))
        }
    } else {
        "<stdin>".to_string()
    };

    if cli.verbose {
        eprintln!("[trdc] Input mode: {user_cmd}");
    }

    eprint!("[trdc] ");
    std::io::stderr().flush()?;

    let (output, _exit_code) = if let Some(ref cmd) = cli.command {
        let command = cmd.clone();
        let args = cli.args.clone();
        tokio::task::spawn_blocking(move || executor::execute(&command, &args)).await??
    } else {
        // Stdin mode - use the data we already captured
        let output = std::mem::take(&mut stdin_data).unwrap_or_default();
        (output, Some(0))
    };

    if cli.verbose {
        eprintln!("[trdc] Output: {} chars", output.len());
    }

    if output.trim().is_empty() {
        println!(r#"{{"summary": "No output", "complete_output_path": ""}}"#);
        return Ok(_exit_code.unwrap_or(0));
    }

    // Apply pre-filtering to reduce token count
    let filtered_output = filter::pre_filter(&output);

    let user_context = cli.context.as_deref();

    if cli.dry_run {
        let chunks = llm::chunk_output(&filtered_output);
        eprintln!(
            "\n[trdc] Dry run - would process {} chunks ({} chars total)\n",
            chunks.len(),
            filtered_output.len()
        );
        eprintln!("[trdc] Full output would be saved to: ~/.local/share/trdc/outputs/");
        return Ok(_exit_code.unwrap_or(0));
    }

    eprintln!("Summarizing...");
    std::io::stderr().flush()?;

    let mut stats = SessionStats::load().unwrap_or_default();
    let input_tokens = estimate_tokens(&filtered_output);
    let llm_success =
        match llm::summarize(&filtered_output, &user_cmd, user_context, &config.llm).await {
            Ok((result, usage, was_streamed)) => {
                // build_output() with was_streamed=true skips the summary since it was already streamed
                println!(
                    "{}",
                    llm::build_output(&result.summary, &result.complete_output_path, was_streamed)
                );

                stats.add_usage(&TokenUsage {
                    input_tokens,
                    output_tokens: usage.output_tokens,
                });

                if let Err(e) = stats.save()
                    && cli.verbose
                {
                    eprintln!("[trdc] Warning: Failed to save stats: {e}");
                }

                if cli.show_gain {
                    let current = TokenUsage {
                        input_tokens,
                        output_tokens: usage.output_tokens,
                    };
                    display_gain(&stats, Some(&current));
                }

                // Append to history on successful summarization
                let entry = history::HistoryEntry {
                    timestamp: chrono::Local::now().to_rfc3339(),
                    command: user_cmd.clone(),
                    summary: result.summary.clone(),
                };
                if let Err(e) = history::append(&entry)
                    && cli.verbose
                {
                    eprintln!("[trdc] Warning: Failed to save history: {e}");
                }

                true
            }
            Err(e) => {
                eprintln!("[trdc] LLM summarization failed: {e} - showing raw output",);
                println!("{output}");
                false
            }
        };

    let final_exit_code = match (_exit_code, llm_success) {
        (Some(code), true) => code,  // Subprocess failed, LLM succeeded
        (Some(code), false) => code, // Subprocess failed, LLM failed
        (None, true) => 0,           // Subprocess succeeded, LLM succeeded
        (None, false) => 1,          // Subprocess succeeded, LLM failed
    };

    Ok(final_exit_code)
}

/// Validates stdin data: checks size limit and emptiness.
/// Returns Some(data) if valid, None if empty/whitespace, or Err on size exceeded.
fn validate_stdin_data(data: String) -> Result<Option<String>, &'static str> {
    if data.trim().is_empty() {
        return Ok(None);
    }
    const MAX_STDIN_SIZE: usize = 10_485_760; // 10MB
    if data.len() > MAX_STDIN_SIZE {
        return Err("stdin exceeds maximum size (10MB)");
    }
    Ok(Some(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_stdin_data_valid() {
        let result = validate_stdin_data("hello world".to_string());
        assert_eq!(result, Ok(Some("hello world".to_string())));
    }

    #[test]
    fn test_validate_stdin_data_empty() {
        let result = validate_stdin_data("".to_string());
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_validate_stdin_data_whitespace_only() {
        let result = validate_stdin_data("   \n\t  ".to_string());
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_validate_stdin_data_too_large() {
        let large_data = "x".repeat(10_485_761); // 10MB + 1 byte
        let result = validate_stdin_data(large_data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "stdin exceeds maximum size (10MB)");
    }

    #[test]
    fn test_validate_stdin_data_at_limit() {
        let data = "x".repeat(10_485_760); // exactly 10MB
        let result = validate_stdin_data(data.clone());
        assert_eq!(result, Ok(Some(data)));
    }
}
