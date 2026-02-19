use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, execute_cargo_subcommand, resolve_package_arg};
use crate::types::CargoFlags;

/// Clean build artifacts
#[derive(Args)]
pub struct CleanCmd {
    /// Package to clean (name or path, e.g. "." for current dir)
    #[arg(value_name = "PACKAGE")]
    pub positional: Option<String>,
    /// Package to clean (defaults to entire workspace/project)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Only clean release artifacts
    #[arg(short, long)]
    pub release: bool,
}

impl Execute for CleanCmd {
    fn execute(&self, project_root: &Path, flags: &CargoFlags) -> Result<ExitCode> {
        let mut args: Vec<OsString> = Vec::new();
        let pkg_arg = self.positional.as_ref().or(self.package.as_ref());
        if let Some(pkg) = pkg_arg
            && let Some(name) = resolve_package_arg(pkg)?
        {
            args.push("-p".into());
            args.push(name.into());
        }
        if self.release {
            args.push("--release".into());
        }
        execute_cargo_subcommand("clean", &args, project_root, flags)
    }
}
