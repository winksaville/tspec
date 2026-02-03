use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{CargoPassthrough, current_package_name};
use crate::all::{print_run_summary, run_all};
use crate::binary::strip_binary;
use crate::cargo_build::build_crate;
use crate::run::run_binary;
use crate::workspace::WorkspaceInfo;

/// Build and run package(s) with a translation spec
#[derive(Args)]
pub struct RunCmd {
    /// Package to run (defaults to current directory or all apps)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Run all apps (even when in a package directory)
    #[arg(short = 'a', long = "all")]
    pub all: bool,
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

impl CargoPassthrough for RunCmd {
    fn subcommand(&self) -> &str {
        "run"
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
                // Run all apps (args not supported for --all)
                let workspace = WorkspaceInfo::discover()?;
                let results = run_all(&workspace, self.tspec.as_deref(), self.release, self.strip);
                Ok(print_run_summary(&results))
            }
            Some(name) => {
                // Build, optionally strip, then run
                let result = build_crate(&name, self.tspec.as_deref(), self.release)?;
                if self.strip {
                    strip_binary(&result.binary_path)?;
                }
                let exit_code = run_binary(&result.binary_path, &self.args)?;
                std::process::exit(exit_code);
            }
        }
    }
}
