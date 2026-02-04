use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, execute_cargo_subcommand};

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

impl Execute for CleanCmd {
    fn execute(&self, project_root: &Path) -> Result<ExitCode> {
        let mut args: Vec<OsString> = Vec::new();
        if let Some(pkg) = &self.package {
            args.push("-p".into());
            args.push(pkg.into());
        }
        if self.release {
            args.push("--release".into());
        }
        execute_cargo_subcommand("clean", &args, project_root)
    }
}
