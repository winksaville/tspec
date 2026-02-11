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
        /// List all workspace packages (even when in a package directory)
        #[arg(short = 'w', long = "workspace")]
        all: bool,
    },
    /// Show a tspec's contents
    Show {
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Show all workspace packages (even when in a package directory)
        #[arg(short = 'w', long = "workspace")]
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
        /// Hash all workspace packages (even when in a package directory)
        #[arg(short = 'w', long = "workspace")]
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
    /// Set a field in a tspec (scalar value or replace entire array)
    ///
    /// For scalars: tspec ts set key value
    /// For arrays: tspec ts set key val1 val2 ...
    Set {
        /// Field key (e.g., "rustc.lto", "linker.args")
        key: String,
        /// Value(s). For scalars, one value. For arrays, each arg is an element.
        #[arg(required = true, allow_hyphen_values = true)]
        value: Vec<String>,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Tspec to modify (defaults to package's tspec.ts.toml)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Remove a field from a tspec (preserves comments)
    Unset {
        /// Key to remove (e.g., "rustc.lto", "panic", "linker.args")
        key: String,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Tspec to modify (defaults to package's tspec.ts.toml)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Add items to an array field (append by default, or insert at position)
    Add {
        /// Field key (must be an array field, e.g., "linker.args")
        key: String,
        /// Items to add
        #[arg(required = true, allow_hyphen_values = true)]
        value: Vec<String>,
        /// Insert at this index instead of appending
        #[arg(short = 'i', long = "index")]
        index: Option<usize>,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Tspec to modify (defaults to package's tspec.ts.toml)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Remove items from an array field (by value or by index)
    Remove {
        /// Field key (must be an array field, e.g., "linker.args")
        key: String,
        /// Items to remove by value (not used with --index)
        #[arg(allow_hyphen_values = true)]
        value: Vec<String>,
        /// Remove item at this index instead of by value
        #[arg(short = 'i', long = "index")]
        index: Option<usize>,
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Tspec to modify (defaults to package's tspec.ts.toml)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Create a versioned backup of a tspec
    Backup {
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Tspec to backup (defaults to package's tspec.ts.toml)
        #[arg(short = 't', long = "tspec")]
        tspec: Option<String>,
    },
    /// Restore a tspec from a versioned backup
    Restore {
        /// Package name (defaults to current directory)
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        /// Backup tspec to restore from (required, e.g., "t2-001-abcd1234.ts.toml")
        #[arg(short = 't', long = "tspec")]
        tspec: String,
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
                key,
                value,
                package,
                tspec,
            } => {
                ts_cmd::set_value(
                    project_root,
                    package.as_deref(),
                    key,
                    value,
                    tspec.as_deref(),
                )?;
            }
            TsCommands::Unset {
                key,
                package,
                tspec,
            } => {
                ts_cmd::unset_value(project_root, package.as_deref(), key, tspec.as_deref())?;
            }
            TsCommands::Add {
                key,
                value,
                index,
                package,
                tspec,
            } => {
                ts_cmd::add_value(
                    project_root,
                    package.as_deref(),
                    key,
                    value,
                    *index,
                    tspec.as_deref(),
                )?;
            }
            TsCommands::Remove {
                key,
                value,
                index,
                package,
                tspec,
            } => {
                ts_cmd::remove_value(
                    project_root,
                    package.as_deref(),
                    key,
                    value,
                    *index,
                    tspec.as_deref(),
                )?;
            }
            TsCommands::Backup { package, tspec } => {
                ts_cmd::backup_tspec(project_root, package.as_deref(), tspec.as_deref())?;
            }
            TsCommands::Restore { package, tspec } => {
                ts_cmd::restore_tspec(project_root, package.as_deref(), tspec)?;
            }
        }
        Ok(ExitCode::SUCCESS)
    }
}
