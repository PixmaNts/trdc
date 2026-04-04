//! Command execution

use anyhow::{Context, Result};
use std::process::Command;

pub fn execute(command: &str, args: &[String]) -> Result<(String, Option<i32>)> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute '{command}'"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stdout}\n{stderr}")
    };

    let exit_code = output.status.code();

    if !output.status.success() && !combined.is_empty() {
        let code = exit_code.unwrap_or(1);
        eprintln!("[trdc] Command exited with code {code}");
    }

    Ok((combined, exit_code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_echo() {
        let result = execute("echo", &["hello".to_string(), "world".to_string()]);
        assert!(result.is_ok());
        let (output, _exit_code) = result.unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn test_execute_invalid_command() {
        let result = execute("nonexistent_command_xyz", &[]);
        assert!(result.is_err());
    }
}
