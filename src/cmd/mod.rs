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

use crate::find_paths::{find_package_dir, find_project_root, get_package_name, is_pop};

/// Trait for command execution.
pub trait Execute {
    fn execute(&self, project_root: &Path) -> Result<ExitCode>;
}

/// Resolve a `-p` argument (path or name) to the actual cargo package name.
/// Returns Some(name) for a package, None if it resolves to a workspace root
/// with no `[package]` section (meaning "operate on all packages").
pub(crate) fn resolve_package_arg(pkg: &str) -> Result<Option<String>> {
    let workspace = find_project_root()?;
    let pkg_dir = find_package_dir(&workspace, pkg)?;
    match get_package_name(&pkg_dir) {
        Ok(name) => Ok(Some(name)),
        Err(_) => Ok(None),
    }
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

/// Check if current directory is a package (has Cargo.toml with [package]).
/// Returns None at a workspace root so that commands default to all-packages mode.
pub(crate) fn current_package_name() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    // At a workspace root, don't treat it as a single package
    if cwd.join("Cargo.toml").exists() && !is_pop(&cwd) {
        return None;
    }
    get_package_name(&cwd).ok()
}
