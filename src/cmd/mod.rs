mod clean;
mod clippy;
mod fmt;
mod test;

pub use clean::CleanCmd;
pub use clippy::ClippyCmd;
pub use fmt::FmtCmd;
pub use test::TestCmd;

use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use crate::find_paths::get_crate_name;

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

/// Check if current directory is a package (has Cargo.toml with [package])
pub(crate) fn current_package_name() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    get_crate_name(&cwd).ok()
}
