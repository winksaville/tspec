use crate::cmd::{
    BuildCmd, CleanCmd, ClippyCmd, CompareCmd, FmtCmd, InstallCmd, RunCmd, TestCmd, TsCmd,
    VersionCmd,
};
use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tspec", version, about = "Translation spec based build system")]
#[command(before_help = concat!("tspec ", env!("CARGO_PKG_VERSION")))]
pub struct Cli {
    /// Increase verbosity (-v for commands/env + cargo -v, -vv for spec details + cargo -vv)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    pub verbose: u8,
    /// Number of parallel jobs to pass to cargo
    #[arg(short = 'j', long = "jobs", global = true)]
    pub jobs: Option<u16>,
    /// Path to Cargo.toml or directory containing one
    #[arg(
        long = "manifest-path",
        visible_alias = "mp",
        global = true,
        value_name = "PATH"
    )]
    pub manifest_path: Option<PathBuf>,
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
