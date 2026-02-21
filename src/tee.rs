//! Tee utility: run a command, print stdout live, collect matching lines.

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, ExitStatus, Stdio};

/// Result of a tee'd command execution.
pub struct TeeResult {
    pub status: ExitStatus,
    pub matched_lines: Vec<String>,
}

/// Spawn a command with piped stdout (stderr inherited),
/// print each stdout line live, and collect lines where
/// `filter(line)` returns true.
pub fn tee_stdout<F>(cmd: &mut Command, filter: F) -> Result<TeeResult>
where
    F: Fn(&str) -> bool,
{
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn command")?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let reader = BufReader::new(stdout);

    let mut matched_lines = Vec::new();

    for line in reader.lines() {
        let line = line.context("failed to read stdout line")?;
        if filter(&line) {
            matched_lines.push(line.clone());
        }
        println!("{}", line);
    }

    let status = child.wait().context("failed to wait for command")?;

    Ok(TeeResult {
        status,
        matched_lines,
    })
}
