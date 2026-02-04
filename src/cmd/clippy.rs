use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, execute_cargo_subcommand};

/// Run clippy lints
#[derive(Args)]
pub struct ClippyCmd {
    /// Package to check (defaults to entire workspace)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Check all packages in workspace
    #[arg(short = 'a', long = "all")]
    pub all: bool,
}

impl Execute for ClippyCmd {
    fn execute(&self, project_root: &Path) -> Result<ExitCode> {
        let mut args: Vec<OsString> = Vec::new();
        if let Some(pkg) = &self.package {
            args.push("-p".into());
            args.push(pkg.into());
        }
        if self.all {
            args.push("--workspace".into());
        }
        execute_cargo_subcommand("clippy", &args, project_root)
    }
}
