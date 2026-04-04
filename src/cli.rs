//! CLI argument parsing

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "trdc",
    about = "Token Reducer CLI - LLM-first context-aware command output summarization",
    version,
    trailing_var_arg = true,
    allow_hyphen_values = true
)]
pub struct Cli {
    /// Run in dry-run mode (show prompt without sending to LLM)
    #[arg(short, long)]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Show token savings summary (like rtk gain)
    #[arg(long = "gain")]
    pub show_gain: bool,

    /// Show command history
    #[arg(short = 'H', long = "history")]
    pub show_history: bool,

    /// Show full history entries (used with --history)
    #[arg(long)]
    pub full: bool,

    /// Custom context/instructions for summarization
    #[arg(long = "context")]
    pub context: Option<String>,

    /// LLM model to use (overrides config)
    #[arg(long)]
    pub model: Option<String>,

    /// Maximum tokens in LLM output (overrides config)
    #[arg(long)]
    pub max_tokens: Option<usize>,

    /// Command to execute (optional if stdin is piped)
    pub command: Option<String>,

    /// Arguments for the command (use -- to separate trdc args)
    pub args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let cli = Cli::parse_from(["trdc", "echo", "hello"]);
        assert_eq!(cli.command, Some("echo".to_string()));
        assert_eq!(cli.args, vec!["hello"]);
        assert!(!cli.dry_run);
        assert!(!cli.verbose);
        assert!(!cli.show_gain);
    }

    #[test]
    fn test_parse_with_context() {
        let cli = Cli::parse_from(["trdc", "--context", "Focus on failures", "cargo", "test"]);
        assert_eq!(cli.command, Some("cargo".to_string()));
        assert_eq!(cli.args, vec!["test"]);
        assert_eq!(cli.context, Some("Focus on failures".to_string()));
    }

    #[test]
    fn test_parse_with_ls_flags() {
        let cli = Cli::parse_from(["trdc", "ls", "-lhr"]);
        assert_eq!(cli.command, Some("ls".to_string()));
        assert_eq!(cli.args, vec!["-lhr"]);
    }

    #[test]
    fn test_parse_with_separator() {
        let cli = Cli::parse_from(["trdc", "--", "ls", "-la"]);
        assert_eq!(cli.command, Some("ls".to_string()));
        assert_eq!(cli.args, vec!["-la"]);
    }

    #[test]
    fn test_parse_with_gain_flag() {
        let cli = Cli::parse_from(["trdc", "--gain", "echo", "test"]);
        assert!(cli.show_gain);
        assert_eq!(cli.command, Some("echo".to_string()));
        assert_eq!(cli.args, vec!["test"]);
    }

    #[test]
    fn test_parse_no_command() {
        let cli = Cli::parse_from(["trdc"]);
        assert!(cli.command.is_none());
        assert!(cli.args.is_empty());
    }
}
