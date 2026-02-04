use anyhow::Result;
use clap::Args;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use super::CargoPassthrough;

/// Print version information
#[derive(Args)]
pub struct VersionCmd;

impl CargoPassthrough for VersionCmd {
    fn subcommand(&self) -> &str {
        unreachable!("VersionCmd does not use cargo passthrough")
    }

    fn args(&self) -> Vec<OsString> {
        unreachable!("VersionCmd does not use cargo passthrough")
    }

    fn execute(&self, _project_root: &Path) -> Result<ExitCode> {
        println!("tspec {}", env!("CARGO_PKG_VERSION"));
        Ok(ExitCode::SUCCESS)
    }
}
