//! Command history tracking for TRDC

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub command: String,
    pub summary: String,
}

fn get_history_path() -> PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    data_dir.join("trdc").join("history.jsonl")
}

pub fn append(entry: &HistoryEntry) -> Result<()> {
    let path = get_history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new().append(true).create(true).open(&path)?;
    let mut writer = std::io::BufWriter::new(file);
    let json = serde_json::to_string(entry)?;
    writeln!(writer, "{}", json)?;
    writer.flush()?;
    Ok(())
}

pub fn load(limit: Option<usize>) -> Result<Vec<HistoryEntry>> {
    let path = get_history_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    #[allow(clippy::never_loop)]
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
            entries.push(entry);
        }
    }
    if let Some(n) = limit {
        let start = entries.len().saturating_sub(n);
        entries = entries[start..].to_vec();
    }
    Ok(entries)
}

pub fn display_history(entries: &[HistoryEntry], full: bool) {
    if entries.is_empty() {
        println!("No history entries.");
        return;
    }
    for entry in entries {
        if full {
            println!("{}", "─".repeat(60));
            println!("Command: {}", entry.command);
            println!("Time: {}", entry.timestamp);
            println!("Summary:\n{}", entry.summary);
        } else {
            let date = &entry.timestamp[..16];
            let summary_preview = if entry.summary.len() > 50 {
                format!("{}...", &entry.summary[..50])
            } else {
                entry.summary.clone()
            };
            let cmd_display = if entry.command == "<stdin>" {
                "<stdin>".to_string()
            } else {
                entry.command.clone()
            };
            println!("{} | {} | {}", date, cmd_display, summary_preview);
        }
    }
    if full {
        println!("{}", "─".repeat(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Helper to create test entry
    fn test_entry(cmd: &str, summary: &str) -> HistoryEntry {
        HistoryEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            command: cmd.to_string(),
            summary: summary.to_string(),
        }
    }

    #[test]
    fn test_history_entry_serialization() {
        let entry = HistoryEntry {
            timestamp: "2026-04-03T14:30:00Z".to_string(),
            command: "cargo test".to_string(),
            summary: "All tests passed".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "cargo test");
        assert_eq!(parsed.summary, "All tests passed");
    }

    #[test]
    fn test_display_history_compact() {
        let entries = vec![
            HistoryEntry {
                timestamp: "2026-04-03T14:30:00+00:00".to_string(),
                command: "cargo test".to_string(),
                summary: "All tests passed".to_string(),
            },
            HistoryEntry {
                timestamp: "2026-04-03T14:31:00+00:00".to_string(),
                command: "<stdin>".to_string(),
                summary: "Short summary".to_string(),
            },
        ];
        display_history(&entries, false);
    }

    #[test]
    fn test_display_history_full() {
        let entries = vec![HistoryEntry {
            timestamp: "2026-04-03T14:30:00+00:00".to_string(),
            command: "cargo test".to_string(),
            summary: "All tests passed".to_string(),
        }];
        display_history(&entries, true);
    }

    #[test]
    fn test_display_history_empty() {
        display_history(&[], false);
    }

    #[test]
    fn test_display_history_long_summary_truncation() {
        let entries = vec![
            HistoryEntry {
                timestamp: "2026-04-03T14:30:00+00:00".to_string(),
                command: "cargo build".to_string(),
                summary: "This is a very long summary that should be truncated in compact mode because it exceeds fifty characters".to_string(),
            },
        ];
        display_history(&entries, false);
    }
}
