use clap::Args;
use std::ffi::OsString;

use super::CargoPassthrough;

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

impl CargoPassthrough for FmtCmd {
    fn subcommand(&self) -> &str {
        "fmt"
    }

    fn args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
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
        args
    }
}
