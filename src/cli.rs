use crate::cmd::{
    BuildCmd, CleanCmd, ClippyCmd, CompareCmd, FmtCmd, InstallCmd, RunCmd, TestCmd, TsCmd,
    VersionCmd,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tspec", version, about = "Translation spec based build system")]
#[command(before_help = concat!("tspec ", env!("CARGO_PKG_VERSION")))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build package(s) with a translation spec
    Build(BuildCmd),
    /// Build and run package(s) with a translation spec
    Run(RunCmd),
    /// Test package(s) with a translation spec
    Test(TestCmd),
    /// Clean build artifacts
    Clean(CleanCmd),
    /// Run clippy lints
    Clippy(ClippyCmd),
    /// Format source code
    Fmt(FmtCmd),
    /// Compare specs for a package (size only)
    Compare(CompareCmd),
    /// Manage translation specs
    Ts(TsCmd),
    /// Print version information
    Version(VersionCmd),
    /// Install a package from a local path
    Install(InstallCmd),
}
