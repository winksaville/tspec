use anyhow::{Context, Result, bail};
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

/// Trait for commands that wrap cargo subcommands with minimal logic.
pub trait CargoPassthrough {
    /// The cargo subcommand to run (e.g., "clean", "clippy", "fmt")
    fn subcommand(&self) -> &str;

    /// Arguments to pass to the cargo subcommand
    fn args(&self) -> Vec<OsString>;

    /// Execute the cargo command with default implementation
    fn execute(&self, project_root: &Path) -> Result<ExitCode> {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg(self.subcommand());
        cmd.args(self.args());
        cmd.current_dir(project_root);
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
#[derive(Args)]
pub struct CleanCmd {
    /// Package to clean (defaults to entire workspace/project)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Only clean release artifacts
    #[arg(short, long)]
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
}
