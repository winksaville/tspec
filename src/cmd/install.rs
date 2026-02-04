use anyhow::{Context, Result, bail};
use clap::Args;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::Execute;

/// Install a package from a local path
#[derive(Args)]
pub struct InstallCmd {
    /// Path to package (relative or absolute)
    #[arg(long)]
    pub path: PathBuf,
    /// Force reinstall even if already installed
    #[arg(short, long)]
    pub force: bool,
}

impl Execute for InstallCmd {
    fn execute(&self, _project_root: &Path) -> Result<ExitCode> {
        let resolved = self
            .path
            .canonicalize()
            .with_context(|| format!("path not found: {}", self.path.display()))?;

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("install").arg("--path").arg(&resolved);
        if self.force {
            cmd.arg("--force");
        }

        let status = cmd.status().context("failed to run cargo install")?;
        if !status.success() {
            bail!("cargo install failed");
        }
        Ok(ExitCode::SUCCESS)
    }
}
