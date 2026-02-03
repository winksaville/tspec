use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Trait for commands that wrap cargo subcommands with minimal logic.
pub trait CargoPassthrough {
    /// The cargo subcommand to run (e.g., "clean", "clippy", "fmt")
    fn subcommand(&self) -> &str;

    /// Arguments to pass to the cargo subcommand
    fn args(&self) -> Vec<OsString>;

    /// Working directory for the command (defaults to current directory)
    fn workdir(&self) -> Option<&Path> {
        None
    }

    /// Execute the cargo command with default implementation
    fn execute(&self) -> Result<ExitCode> {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg(self.subcommand());
        cmd.args(self.args());
        if let Some(dir) = self.workdir() {
            cmd.current_dir(dir);
        }
        let status = cmd
            .status()
            .with_context(|| format!("failed to run cargo {}", self.subcommand()))?;
        if status.success() {
            Ok(ExitCode::SUCCESS)
        } else {
            bail!("cargo {} failed", self.subcommand());
        }
    }
}

/// Clean build artifacts
pub struct CleanCmd {
    pub workspace: PathBuf,
    pub package: Option<String>,
    pub release: bool,
}

impl CargoPassthrough for CleanCmd {
    fn subcommand(&self) -> &str {
        "clean"
    }

    fn args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        if let Some(pkg) = &self.package {
            args.push("-p".into());
            args.push(pkg.into());
        }
        if self.release {
            args.push("--release".into());
        }
        args
    }

    fn workdir(&self) -> Option<&Path> {
        Some(&self.workspace)
    }
}
