use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::Execute;
use crate::compare::compare_specs;
use crate::find_paths::{find_package_dir, find_tspecs};

/// Compare specs for a package (size only)
#[derive(Args)]
pub struct CompareCmd {
    /// Package to compare (required)
    #[arg(short = 'p', long = "package")]
    pub package: String,
    /// Spec file(s) or glob pattern(s) (defaults to tspec* pattern)
    #[arg(short = 't', long = "tspec", action = clap::ArgAction::Append)]
    pub tspec: Vec<String>,
    /// Release build
    #[arg(short, long)]
    pub release: bool,
    /// Strip symbols before comparing sizes
    #[arg(short, long)]
    pub strip: bool,
}

impl Execute for CompareCmd {
    fn execute(&self, project_root: &Path) -> Result<ExitCode> {
        let package_dir = find_package_dir(project_root, &self.package)?;
        let spec_paths = find_tspecs(&package_dir, &self.tspec)?;
        compare_specs(&self.package, &spec_paths, self.release, self.strip)?;
        Ok(ExitCode::SUCCESS)
    }
}
