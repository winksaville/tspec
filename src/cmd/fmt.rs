use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, execute_cargo_subcommand};

/// Format source code
#[derive(Args)]
pub struct FmtCmd {
    /// Package to format (defaults to entire workspace)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Format all packages in workspace
    #[arg(short = 'a', long = "all")]
    pub all: bool,
    /// Check formatting without modifying files
    #[arg(short, long)]
    pub check: bool,
}

impl Execute for FmtCmd {
    fn execute(&self, project_root: &Path) -> Result<ExitCode> {
        let mut args: Vec<OsString> = Vec::new();
        if let Some(pkg) = &self.package {
            args.push("-p".into());
            args.push(pkg.into());
        }
        if self.all {
            args.push("--all".into());
        }
        if self.check {
            args.push("--check".into());
        }
        execute_cargo_subcommand("fmt", &args, project_root)
    }
}
