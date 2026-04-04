//! Output pre-filtering for TRDC
//!
//! This module provides functions to clean and compress command output before
//! sending it to the LLM for summarization.

use regex::Regex;

/// Remove ANSI escape sequences from text.
///
/// Handles both CSI (Control Sequence Introducer) and OSC (Operating System Command) sequences.
pub fn strip_ansi(input: &str) -> String {
    // Pattern for CSI sequences: \x1b[...letter
    let csi = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    // Pattern for OSC sequences: \x1b]...BEL or \x1b]...\x1b\\
    let osc = Regex::new(r"\x1b\].*?(?:\x07|\x1b\\)").unwrap();

    let result = csi.replace_all(input, "");
    osc.replace_all(&result, "").to_string()
}

/// Collapse 5+ identical consecutive lines into `[N identical lines collapsed]`.
///
/// When a line repeats `threshold` or more times consecutively, all but the first
/// instance are replaced with a collapse marker.
pub fn collapse_duplicates(input: &str, threshold: usize) -> String {
    if threshold == 0 {
        return input.to_string();
    }

    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let mut result = Vec::new();
    let mut current_line = lines[0];
    let mut count = 1;

    for &line in &lines[1..] {
        if line == current_line {
            count += 1;
        } else {
            // Output accumulated lines
            if count >= threshold {
                result.push(current_line.to_string());
                result.push(format!("[{} identical lines collapsed]", count - 1));
            } else {
                for _ in 0..count {
                    result.push(current_line.to_string());
                }
            }
            current_line = line;
            count = 1;
        }
    }

    // Handle last group
    if count >= threshold {
        result.push(current_line.to_string());
        result.push(format!("[{} identical lines collapsed]", count - 1));
    } else {
        for _ in 0..count {
            result.push(current_line.to_string());
        }
    }

    result.join("\n")
}

/// Trim trailing whitespace from each line and remove leading/trailing blank lines.
pub fn trim_whitespace(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();

    // Trim trailing whitespace from each line
    let trimmed: Vec<String> = lines.iter().map(|l| l.trim_end().to_string()).collect();

    // Find first non-empty line
    let start = trimmed.iter().position(|l| !l.is_empty());
    // Find last non-empty line
    let end = trimmed.iter().rposition(|l| !l.is_empty());

    match (start, end) {
        (Some(s), Some(e)) => trimmed[s..=e].join("\n"),
        _ => String::new(), // All lines are empty
    }
}

/// Chain all filters: strip_ansi → collapse_duplicates → trim_whitespace
///
/// Applies filters in the optimal order to reduce token count before LLM processing.
pub fn pre_filter(input: &str) -> String {
    let result = strip_ansi(input);
    let result = collapse_duplicates(&result, 5); // threshold hardcoded to 5
    trim_whitespace(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_csi() {
        let input = "\x1b[31mRed\x1b[0m text";
        assert_eq!(strip_ansi(input), "Red text");
    }

    #[test]
    fn test_strip_ansi_osc() {
        let input = "\x1b]0;Title\x07Text";
        assert_eq!(strip_ansi(input), "Text");
    }

    #[test]
    fn test_strip_ansi_complex() {
        let input = "\x1b[1;32m\x1b[4mBold Green Underline\x1b[0m";
        assert_eq!(strip_ansi(input), "Bold Green Underline");
    }

    #[test]
    fn test_collapse_duplicates_below_threshold() {
        let input = "line\nline\nline\nline";
        let result = collapse_duplicates(input, 5);
        assert_eq!(result, "line\nline\nline\nline");
    }

    #[test]
    fn test_collapse_duplicates_at_threshold() {
        let input = "line\nline\nline\nline\nline";
        let result = collapse_duplicates(input, 5);
        assert!(result.contains("[4 identical lines collapsed]"));
        assert!(result.starts_with("line\n"));
    }

    #[test]
    fn test_collapse_duplicates_above_threshold() {
        let input = "line\nline\nline\nline\nline\nline\nother";
        let result = collapse_duplicates(input, 5);
        assert!(result.contains("[5 identical lines collapsed]"));
        assert!(result.contains("other"));
    }

    #[test]
    fn test_collapse_duplicates_multiple_groups() {
        let input = "a\na\na\na\na\na\nb\nb\nb\nb\nb\nc";
        let result = collapse_duplicates(input, 5);
        assert!(result.contains("[5 identical lines collapsed]")); // for 'a'
        assert!(result.contains("[4 identical lines collapsed]")); // for 'b'
        assert!(result.contains("c"));
    }

    #[test]
    fn test_collapse_duplicates_empty() {
        assert_eq!(collapse_duplicates("", 5), "");
    }

    #[test]
    fn test_collapse_duplicates_zero_threshold() {
        let input = "line\nline\nline";
        let result = collapse_duplicates(input, 0);
        assert_eq!(result, input);
    }

    #[test]
    fn test_trim_whitespace_trailing() {
        let input = "line1  \nline2\t\nline3   ";
        let result = trim_whitespace(input);
        assert_eq!(result, "line1\nline2\nline3");
    }

    #[test]
    fn test_trim_whitespace_leading_trailing_blanks() {
        let input = "\n\nline1\nline2\n\n\n";
        let result = trim_whitespace(input);
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn test_trim_whitespace_all_blank() {
        let input = "\n\n  \n\t\n";
        let result = trim_whitespace(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_pre_filter_integration() {
        let input = "\n\n\x1b[31mA\nA\nA\nA\nA\nA\nB\x1b[0m  \n\n";
        let result = pre_filter(input);
        assert!(!result.contains("\x1b"));
        assert!(result.contains("[5 identical lines collapsed]"));
        assert!(!result.starts_with('\n'));
        assert!(!result.ends_with('\n'));
    }

    #[test]
    fn test_pre_filter_empty() {
        assert_eq!(pre_filter(""), "");
    }

    #[test]
    fn test_pre_filter_no_changes_needed() {
        let input = "line1\nline2\nline3";
        let result = pre_filter(input);
        assert_eq!(result, "line1\nline2\nline3");
    }
}
