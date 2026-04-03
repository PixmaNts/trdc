//! TRDC - Token Reducer CLI
//!
//! LLM-first context-aware command output summarization.

mod cli;
mod config;
mod executor;
mod llm;
mod tracking;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use config::Config;
use std::io::Write;
use tracking::{SessionStats, TokenUsage, display_gain, estimate_tokens};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

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

    let user_cmd = if cli.args.is_empty() {
        cli.command.clone()
    } else {
        format!("{} {}", cli.command, cli.args.join(" "))
    };

    if cli.verbose {
        eprintln!("[trdc] Executing command: {user_cmd}");
    }

    eprint!("[trdc] ");
    std::io::stderr().flush()?;

    let output = executor::execute(&cli.command, &cli.args)?;

    if cli.verbose {
        eprintln!("[trdc] Output: {} chars", output.len());
    }

    if output.trim().is_empty() {
        println!(r#"{{"summary": "No output", "complete_output_path": ""}}"#);
        return Ok(());
    }

    let user_context = cli.context.as_deref();

    if cli.dry_run {
        let chunks = llm::chunk_output(&output);
        eprintln!(
            "\n[trdc] Dry run - would process {} chunks ({} chars total)\n",
            chunks.len(),
            output.len()
        );
        eprintln!("[trdc] Full output would be saved to: ~/.local/share/trdc/outputs/");
        return Ok(());
    }

    eprintln!("Summarizing...");
    std::io::stderr().flush()?;

    let mut stats = SessionStats::load().unwrap_or_default();
    let input_tokens = estimate_tokens(&output);

    match llm::summarize(&output, &user_cmd, user_context, &config.llm) {
        Ok((result, usage)) => {
            println!(
                "{}",
                llm::build_output(&result.summary, &result.complete_output_path)
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
        }
        Err(e) => {
            eprintln!("[trdc] LLM summarization failed: {e} - showing raw output",);
            println!("{output}");
        }
    }

    Ok(())
}
