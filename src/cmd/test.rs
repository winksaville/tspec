use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{CargoPassthrough, current_package_name};
use crate::all::{print_test_summary, test_all};
use crate::testing::test_crate;
use crate::workspace::WorkspaceInfo;

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
