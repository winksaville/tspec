use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::Path;
use std::process::ExitCode;

use super::Execute;
use crate::ts_cmd::{self, SetOp};

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
    /// Set a value in a tspec (modifies in place, preserves comments)
    ///
    /// Use key=value to replace, key+=value to append to arrays, key-=value to remove from arrays.
    Set {
        /// Assignment: key=value, key+=value (append), or key-=value (remove)
        assignment: String,
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
                assignment,
                package,
                tspec,
            } => {
                let (key, value, op) = parse_assignment(assignment)?;
                ts_cmd::set_value(
                    project_root,
                    package.as_deref(),
                    &key,
                    &value,
                    op,
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

/// Parse an assignment string into (key, value, op).
/// Supports: `key=value`, `key+=value`, `key-=value`
fn parse_assignment(assignment: &str) -> Result<(String, String, SetOp)> {
    // Check for += and -= before plain =
    if let Some((key, value)) = assignment.split_once("+=") {
        return Ok((key.to_string(), value.to_string(), SetOp::Append));
    }
    if let Some((key, value)) = assignment.split_once("-=") {
        return Ok((key.to_string(), value.to_string(), SetOp::Remove));
    }
    if let Some((key, value)) = assignment.split_once('=') {
        return Ok((key.to_string(), value.to_string(), SetOp::Replace));
    }
    anyhow::bail!(
        "invalid assignment '{}': expected key=value, key+=value, or key-=value",
        assignment
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_assignment() {
        let (key, value, op) = parse_assignment("rustc.lto=true").unwrap();
        assert_eq!(key, "rustc.lto");
        assert_eq!(value, "true");
        assert_eq!(op, SetOp::Replace);
    }

    #[test]
    fn parse_append_assignment() {
        let (key, value, op) = parse_assignment("linker.args+=-Wl,--gc-sections").unwrap();
        assert_eq!(key, "linker.args");
        assert_eq!(value, "-Wl,--gc-sections");
        assert_eq!(op, SetOp::Append);
    }

    #[test]
    fn parse_remove_assignment() {
        let (key, value, op) = parse_assignment("linker.args-=-static").unwrap();
        assert_eq!(key, "linker.args");
        assert_eq!(value, "-static");
        assert_eq!(op, SetOp::Remove);
    }

    #[test]
    fn parse_array_bracket_assignment() {
        let (key, value, op) = parse_assignment(r#"linker.args=["-static","-nostdlib"]"#).unwrap();
        assert_eq!(key, "linker.args");
        assert_eq!(value, r#"["-static","-nostdlib"]"#);
        assert_eq!(op, SetOp::Replace);
    }

    #[test]
    fn parse_no_equals_errors() {
        assert!(parse_assignment("no-equals").is_err());
    }
}
