use clap::Args;
use std::ffi::OsString;

use super::CargoPassthrough;

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

impl CargoPassthrough for CleanCmd {
    fn subcommand(&self) -> &str {
        "clean"
    }

    fn args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        if let Some(pkg) = &self.package {
            args.push("-p".into());
            args.push(pkg.into());
        }
        if self.release {
            args.push("--release".into());
        }
        args
    }
}
