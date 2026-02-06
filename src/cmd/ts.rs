use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::Path;
use std::process::ExitCode;

use super::Execute;
use crate::ts_cmd;

/// Manage translation specs
#[derive(Args)]
pub struct TsCmd {
    #[command(subcommand)]
    command: TsCommands,
}

#[derive(Subcommand)]
pub enum TsCommands {
    /// List tspec files in workspace or for a specific package
    List {
        /// Package to list specs for (defaults to current directory or all packages)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// List all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
    },
    /// Show a tspec's contents
    Show {
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Show all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Tspec name (defaults to all tspec files)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Show the content hash of a tspec
    Hash {
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Hash all packages (even when in a package directory)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Tspec name (defaults to package's tspec file)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Create a new tspec file
    New {
        /// Name for the new tspec (default: "tspec")
        #[arg(default_value = "tspec")]
        name: String,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Copy from existing tspec (package/spec or just spec name in same package)
        #[arg(short = 'f', long = "from")]
        from: Option<String>,
    },
    /// Set a scalar value in a tspec (creates versioned copy)
    Set {
        /// Key=value pair (e.g., "strip=symbols", "panic=abort", "rustc.lto=true")
        assignment: String,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Tspec to modify (defaults to package's tspec.ts.toml)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
}

impl Execute for TsCmd {
    fn execute(&self, project_root: &Path) -> Result<ExitCode> {
        match &self.command {
            TsCommands::List { package, all } => {
                ts_cmd::list_tspecs(project_root, package.as_deref(), *all)?;
            }
            TsCommands::Show {
                package,
                all,
                tspec,
            } => {
                ts_cmd::show_tspec(project_root, package.as_deref(), *all, tspec.as_deref())?;
            }
            TsCommands::Hash {
                package,
                all,
                tspec,
            } => {
                ts_cmd::hash_tspec(project_root, package.as_deref(), *all, tspec.as_deref())?;
            }
            TsCommands::New {
                name,
                package,
                from,
            } => {
                ts_cmd::new_tspec(project_root, package.as_deref(), name, from.as_deref())?;
            }
            TsCommands::Set {
                assignment,
                package,
                tspec,
            } => {
                let (key, value) = assignment.split_once('=').ok_or_else(|| {
                    anyhow::anyhow!("invalid assignment '{}': expected key=value", assignment)
                })?;
                ts_cmd::set_value(
                    project_root,
                    package.as_deref(),
                    key,
                    value,
                    tspec.as_deref(),
                )?;
            }
        }
        Ok(ExitCode::SUCCESS)
    }
}
