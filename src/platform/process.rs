use std::process::Command;
use crate::error::{NServoError, Result};

/// Safe process executor used for external integrations such as `pass`.
pub struct ProcessRunner;

impl ProcessRunner {
    /// Executes a system command with arguments and captures trimmed stdout.
    pub fn run_command(program: &str, args: &[&str]) -> Result<String> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| NServoError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(NServoError::Plugin {
                name: program.to_string(),
                reason: format!("Command exited with status {}: {}", output.status, stderr),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout)
    }

    /// Checks if a program exists on the system PATH.
    pub fn binary_exists(program: &str) -> bool {
        Command::new("which")
            .arg(program)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
