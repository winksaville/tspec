use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, current_package_name};
use crate::all::{print_run_summary, run_all};
use crate::binary::strip_binary;
use crate::cargo_build::build_package;
use crate::run::run_binary;
use crate::workspace::WorkspaceInfo;

/// Build and run package(s) with a translation spec
#[derive(Args)]
pub struct RunCmd {
    /// Package to run (defaults to current directory or all apps)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Run all workspace apps (even when in a package directory)
    #[arg(short = 'w', long = "workspace")]
    pub workspace: bool,
    /// Translation spec to use (defaults to package's tspec file)
    #[arg(short = 't', long = "tspec")]
    pub tspec: Option<String>,
    /// Release build
    #[arg(short, long)]
    pub release: bool,
    /// Strip symbols from binary before running
    #[arg(short, long)]
    pub strip: bool,
    /// Arguments to pass to the binary (after --)
    #[arg(last = true)]
    pub args: Vec<String>,
}

impl Execute for RunCmd {
    fn execute(&self, _project_root: &Path) -> Result<ExitCode> {
        // Resolve package: --workspace > -p PKG > cwd > all
        let resolved = if self.workspace {
            None
        } else {
            self.package.clone().or_else(current_package_name)
        };

        match resolved {
            None => {
                // Run all apps (args not supported for --workspace)
                let workspace = WorkspaceInfo::discover()?;
                let results = run_all(&workspace, self.tspec.as_deref(), self.release, self.strip);
                Ok(print_run_summary(&results))
            }
            Some(name) => {
                // Build, optionally strip, then run
                let result = build_package(&name, self.tspec.as_deref(), self.release)?;
                if self.strip {
                    strip_binary(&result.binary_path)?;
                }
                let exit_code = run_binary(&result.binary_path, &self.args)?;
                std::process::exit(exit_code);
            }
        }
    }
}
