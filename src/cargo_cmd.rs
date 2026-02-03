use anyhow::{Context, Result, bail};
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use crate::all::{print_test_summary, test_all};
use crate::find_paths::get_crate_name;
use crate::testing::test_crate;
use crate::workspace::WorkspaceInfo;

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

/// Test package(s) with a translation spec
#[derive(Args)]
pub struct TestCmd {
    /// Package to test (defaults to current directory or all packages)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Test all packages (even when in a package directory)
    #[arg(short = 'a', long = "all")]
    pub all: bool,
    /// Translation spec to use (defaults to package's tspec file)
    #[arg(short = 't', long = "tspec")]
    pub tspec: Option<String>,
    /// Release build
    #[arg(short, long)]
    pub release: bool,
    /// Stop on first failure
    #[arg(short, long)]
    pub fail_fast: bool,
}

impl CargoPassthrough for TestCmd {
    fn subcommand(&self) -> &str {
        "test"
    }

    fn args(&self) -> Vec<OsString> {
        // Not used - execute() builds its own command
        vec![]
    }

    fn execute(&self, _project_root: &Path) -> Result<ExitCode> {
        // Resolve package: --all > -p PKG > cwd > all
        let resolved = if self.all {
            None
        } else {
            self.package.clone().or_else(current_package_name)
        };

        match resolved {
            None => {
                // Test all packages
                let workspace = WorkspaceInfo::discover()?;
                let results = test_all(
                    &workspace,
                    self.tspec.as_deref(),
                    self.release,
                    self.fail_fast,
                );
                Ok(print_test_summary(&results))
            }
            Some(name) => {
                test_crate(&name, self.tspec.as_deref(), self.release)?;
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

/// Check if current directory is a package (has Cargo.toml with [package])
fn current_package_name() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    get_crate_name(&cwd).ok()
}
