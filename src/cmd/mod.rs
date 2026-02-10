mod build;
mod clean;
mod clippy;
mod compare;
mod fmt;
mod install;
mod run;
mod test;
mod ts;
mod version;

pub use build::BuildCmd;
pub use clean::CleanCmd;
pub use clippy::ClippyCmd;
pub use compare::CompareCmd;
pub use fmt::FmtCmd;
pub use install::InstallCmd;
pub use run::RunCmd;
pub use test::TestCmd;
pub use ts::TsCmd;
pub use version::VersionCmd;

use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use crate::find_paths::get_package_name;

/// Trait for command execution.
pub trait Execute {
    fn execute(&self, project_root: &Path) -> Result<ExitCode>;
}

/// Helper for commands that execute a cargo subcommand.
pub fn execute_cargo_subcommand(
    subcommand: &str,
    args: &[OsString],
    project_root: &Path,
) -> Result<ExitCode> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg(subcommand);
    cmd.args(args);
    cmd.current_dir(project_root);
    let status = cmd
        .status()
        .with_context(|| format!("failed to run cargo {}", subcommand))?;
    if status.success() {
        Ok(ExitCode::SUCCESS)
    } else {
        bail!("cargo {} failed", subcommand);
    }
}

/// Check if current directory is a package (has Cargo.toml with [package])
pub(crate) fn current_package_name() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    get_package_name(&cwd).ok()
}
