use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Run a binary with optional arguments and return its exit code
pub fn run_binary(binary_path: &Path, args: &[String]) -> Result<i32> {
    println!("Running {}", binary_path.display());
    let status = Command::new(binary_path)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {}", binary_path.display()))?;

    Ok(status.code().unwrap_or(1))
}
