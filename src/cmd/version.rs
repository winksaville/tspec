use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::Execute;
use crate::types::Verbosity;

/// Print version information
#[derive(Args)]
pub struct VersionCmd;

impl Execute for VersionCmd {
    fn execute(&self, _project_root: &Path, _verbosity: Verbosity) -> Result<ExitCode> {
        println!("tspec {}", env!("CARGO_PKG_VERSION"));
        Ok(ExitCode::SUCCESS)
    }
}
