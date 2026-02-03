use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{CargoPassthrough, current_package_name};
use crate::all::{build_all, print_summary};
use crate::binary::strip_binary;
use crate::cargo_build::build_crate;
use crate::workspace::WorkspaceInfo;

/// Build package(s) with a translation spec
#[derive(Args)]
pub struct BuildCmd {
    /// Package to build (defaults to current directory or all packages)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Build all packages (even when in a package directory)
    #[arg(short = 'a', long = "all")]
    pub all: bool,
    /// Translation spec to use (defaults to package's tspec file)
    #[arg(short = 't', long = "tspec")]
    pub tspec: Option<String>,
    /// Release build
    #[arg(short, long)]
    pub release: bool,
    /// Strip symbols from binary after build
    #[arg(short, long)]
    pub strip: bool,
    /// Stop on first failure (for all-packages mode)
    #[arg(short, long)]
    pub fail_fast: bool,
}

impl CargoPassthrough for BuildCmd {
    fn subcommand(&self) -> &str {
        "build"
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
                // Build all packages
                let workspace = WorkspaceInfo::discover()?;
                let results = build_all(
                    &workspace,
                    self.tspec.as_deref(),
                    self.release,
                    self.strip,
                    self.fail_fast,
                );
                Ok(print_summary(&results))
            }
            Some(name) => {
                let result = build_crate(&name, self.tspec.as_deref(), self.release)?;
                if self.strip {
                    strip_binary(&result.binary_path)?;
                }
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}
