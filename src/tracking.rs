//! Token tracking for TRDC
//!
//! Tracks tokens sent to and received from LLM, calculating compression savings.

//!
//! The savings are calculated as: (`input_tokens` - `output_tokens`) / `input_tokens` * 100
//!
//! Example:
//! ```
//! Input:  1000 `tokens` (command output)
//! LLM:   200 `tokens` (summary)
//! Saved:  800 `tokens` (80% reduction)
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStats {
    pub total_input_tokens: usize,
    pub total_output_tokens: usize,
    pub commands_count: usize,
}
impl SessionStats {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_usage(&mut self, usage: &TokenUsage) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.commands_count += 1;
    }
    #[allow(clippy::cast_precision_loss)]
    pub fn savings_pct(&self) -> f64 {
        if self.total_input_tokens == 0 {
            return 0.0;
        }
        let saved = self
            .total_input_tokens
            .saturating_sub(self.total_output_tokens);
        (saved as f64 / self.total_input_tokens as f64) * 100.0
    }
    pub fn saved_tokens(&self) -> usize {
        self.total_input_tokens
            .saturating_sub(self.total_output_tokens)
    }
    pub fn load() -> Result<Self> {
        let path = get_stats_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let stats: SessionStats = serde_json::from_str(&content).unwrap_or_default();
            Ok(stats)
        } else {
            Ok(Self::new())
        }
    }
    pub fn save(&self) -> Result<()> {
        let path = get_stats_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
    #[allow(dead_code)]
    pub fn reset() -> Result<()> {
        let path = get_stats_path();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}
fn get_stats_path() -> PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    data_dir.join("trdc").join("stats.json")
}
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() as f64 / 4.0).ceil() as usize
}
pub fn display_gain(stats: &SessionStats, current_usage: Option<&TokenUsage>) {
    eprintln!();
    eprintln!("╔══════════════════════════════════════════════════════════╗");
    eprintln!("║              TRDC Token Savings Summary                  ║");
    eprintln!("╠══════════════════════════════════════════════════════════╣");
    if let Some(usage) = current_usage {
        eprintln!("║  Current Command:                                        ║");
        eprintln!(
            "║    Input (to LLM):    {:>8} tokens                    ║",
            usage.input_tokens
        );
        eprintln!(
            "║    Output (from LLM): {:>8} tokens                    ║",
            usage.output_tokens
        );
        let current_saved = usage.input_tokens.saturating_sub(usage.output_tokens);
        let current_pct = if usage.input_tokens > 0 {
            #[allow(clippy::cast_precision_loss)]
            let saved = current_saved as f64;
            #[allow(clippy::cast_precision_loss)]
            let total = usage.input_tokens as f64;
            (saved / total) * 100.0
        } else {
            0.0
        };
        eprintln!(
            "║    Saved:             {current_saved:>8} tokens ({current_pct:>5.1}%)          ║",
        );
        eprintln!("╠══════════════════════════════════════════════════════════╣");
    }
    eprintln!("║  Session Total:                                          ║");
    eprintln!(
        "║    Input (to LLM):    {:>8} tokens                    ║",
        stats.total_input_tokens
    );
    eprintln!(
        "║    Output (from LLM): {:>8} tokens                    ║",
        stats.total_output_tokens
    );
    eprintln!(
        "║    Saved:             {:>8} tokens ({:>5.1}%)          ║",
        stats.saved_tokens(),
        stats.savings_pct()
    );
    eprintln!(
        "║    Commands:          {:>8}                          ║",
        stats.commands_count
    );
    eprintln!("╚══════════════════════════════════════════════════════════╝");
    eprintln!();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_session_stats() {
        let mut stats = SessionStats::new();
        stats.add_usage(&TokenUsage {
            input_tokens: 1000,
            output_tokens: 200,
        });
        stats.add_usage(&TokenUsage {
            input_tokens: 500,
            output_tokens: 100,
        });
        assert_eq!(stats.total_input_tokens, 1500);
        assert_eq!(stats.total_output_tokens, 300);
        assert_eq!(stats.saved_tokens(), 1200);
        assert_eq!(stats.commands_count, 2);
    }
    #[test]
    fn test_savings_pct() {
        let mut stats = SessionStats::new();
        stats.add_usage(&TokenUsage {
            input_tokens: 1000,
            output_tokens: 200,
        });
        let expected_pct = (800.0 / 1000.0) * 100.0;
        assert!((stats.savings_pct() - expected_pct).abs() < 0.01);
    }
    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }
}
