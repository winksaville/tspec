use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, execute_cargo_subcommand, resolve_package_arg};
use crate::types::Verbosity;

/// Format source code
#[derive(Args)]
pub struct FmtCmd {
    /// Package to format (name or path, e.g. "." for current dir)
    #[arg(value_name = "PACKAGE")]
    pub positional: Option<String>,
    /// Package to format (defaults to entire workspace)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Format all workspace packages
    #[arg(short = 'w', long = "workspace")]
    pub workspace: bool,
    /// Check formatting without modifying files
    #[arg(short, long)]
    pub check: bool,
}

impl Execute for FmtCmd {
    fn execute(&self, project_root: &Path, _verbosity: Verbosity) -> Result<ExitCode> {
        let mut args: Vec<OsString> = Vec::new();
        let pkg_arg = self.positional.as_ref().or(self.package.as_ref());
        if let Some(pkg) = pkg_arg
            && let Some(name) = resolve_package_arg(pkg)?
        {
            args.push("-p".into());
            args.push(name.into());
        }
        if self.workspace {
            args.push("--all".into());
        }
        if self.check {
            args.push("--check".into());
        }
        execute_cargo_subcommand("fmt", &args, project_root)
    }
}
