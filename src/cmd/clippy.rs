use clap::Args;
use std::ffi::OsString;

use super::CargoPassthrough;

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

impl CargoPassthrough for ClippyCmd {
    fn subcommand(&self) -> &str {
        "clippy"
    }

    fn args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        if let Some(pkg) = &self.package {
            args.push("-p".into());
            args.push(pkg.into());
        }
        if self.all {
            args.push("--workspace".into());
        }
        args
    }
}
