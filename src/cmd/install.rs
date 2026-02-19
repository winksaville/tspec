use anyhow::{Context, Result, bail};
use clap::Args;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::Execute;
use crate::find_paths::get_package_name;
use crate::types::CargoFlags;

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
    fn execute(&self, _project_root: &Path, flags: &CargoFlags) -> Result<ExitCode> {
        let resolved = self
            .path
            .canonicalize()
            .with_context(|| format!("path not found: {}", self.path.display()))?;

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("install").arg("--path").arg(&resolved);

        // Pass package name if we can determine it (needed for workspaces)
        if let Ok(name) = get_package_name(&resolved) {
            cmd.arg(&name);
        }

        if self.force {
            cmd.arg("--force");
        }

        flags.apply_to_command(&mut cmd);

        let status = cmd.status().context("failed to run cargo install")?;
        if !status.success() {
            bail!("cargo install failed");
        }
        Ok(ExitCode::SUCCESS)
    }
}
