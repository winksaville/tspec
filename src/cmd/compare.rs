use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, current_package_name, resolve_package_arg};
use crate::all::{compare_all, print_compare_summary};
use crate::compare::{compare_specs, print_comparison};
use crate::find_paths::{find_tspecs, get_package_name, resolve_package_dir};
use crate::types::Verbosity;
use crate::workspace::WorkspaceInfo;

/// Compare specs for a package (size only)
#[derive(Args)]
pub struct CompareCmd {
    /// Package to compare (name or path, e.g. "." for current dir)
    #[arg(value_name = "PACKAGE")]
    pub positional: Option<String>,
    /// Package to compare (defaults to current directory)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Compare all workspace packages (even when in a package directory)
    #[arg(short = 'w', long = "workspace")]
    pub workspace: bool,
    /// Spec file(s) or glob pattern(s) (defaults to tspec* pattern)
    #[arg(short = 't', long = "tspec", num_args = 1..)]
    pub tspec: Vec<String>,
    /// Stop on first failure (for all-packages mode)
    #[arg(short, long)]
    pub fail_fast: bool,
}

impl Execute for CompareCmd {
    fn execute(&self, project_root: &Path, verbosity: Verbosity) -> Result<ExitCode> {
        // Resolve package: --workspace > -p/positional PKG > cwd > all
        let resolved = if self.workspace {
            None
        } else {
            match self.positional.as_deref().or(self.package.as_deref()) {
                Some(pkg) => resolve_package_arg(pkg)?,
                None => current_package_name(),
            }
        };

        match resolved {
            None => {
                let workspace = WorkspaceInfo::discover()?;
                let results = compare_all(&workspace, &self.tspec, self.fail_fast, verbosity);
                Ok(print_compare_summary(workspace.name(), &results))
            }
            Some(pkg_name) => {
                let package_dir = resolve_package_dir(project_root, Some(&pkg_name))?;
                let pkg_name = get_package_name(&package_dir)?;
                let spec_paths = if self.tspec.is_empty() {
                    find_tspecs(&package_dir, &self.tspec).unwrap_or_default()
                } else {
                    find_tspecs(&package_dir, &self.tspec)?
                };
                let results = compare_specs(&pkg_name, &spec_paths, verbosity)?;
                print_comparison(&pkg_name, &results);
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    fn parse(args: &[&str]) -> CompareCmd {
        let mut full = vec!["tspec", "compare"];
        full.extend_from_slice(args);
        let cli = Cli::try_parse_from(full).unwrap();
        match cli.command {
            crate::cli::Commands::Compare(cmd) => cmd,
            _ => panic!("expected Compare command"),
        }
    }

    #[test]
    fn package_optional() {
        let cmd = parse(&[]);
        assert!(cmd.package.is_none());
    }

    #[test]
    fn package_explicit() {
        let cmd = parse(&["-p", "myapp"]);
        assert_eq!(cmd.package.as_deref(), Some("myapp"));
    }

    #[test]
    fn tspec_empty_by_default() {
        let cmd = parse(&[]);
        assert!(cmd.tspec.is_empty());
    }

    #[test]
    fn tspec_single_file() {
        let cmd = parse(&["-t", "foo.ts.toml"]);
        assert_eq!(cmd.tspec, vec!["foo.ts.toml"]);
    }

    #[test]
    fn tspec_multiple_values_one_flag() {
        // Simulates shell-expanded glob: -t file1 file2
        let cmd = parse(&["-t", "a.ts.toml", "b.ts.toml"]);
        assert_eq!(cmd.tspec, vec!["a.ts.toml", "b.ts.toml"]);
    }

    #[test]
    fn tspec_multiple_flags() {
        let cmd = parse(&["-t", "a.ts.toml", "-t", "b.ts.toml"]);
        assert_eq!(cmd.tspec, vec!["a.ts.toml", "b.ts.toml"]);
    }

    #[test]
    fn all_flags_together() {
        let cmd = parse(&["-p", "myapp", "-t", "spec.ts.toml"]);
        assert_eq!(cmd.package.as_deref(), Some("myapp"));
        assert_eq!(cmd.tspec, vec!["spec.ts.toml"]);
    }

    #[test]
    fn workspace_flag_short() {
        let cmd = parse(&["-w"]);
        assert!(cmd.workspace);
    }

    #[test]
    fn workspace_flag_long() {
        let cmd = parse(&["--workspace"]);
        assert!(cmd.workspace);
    }

    #[test]
    fn workspace_default_false() {
        let cmd = parse(&[]);
        assert!(!cmd.workspace);
    }

    #[test]
    fn fail_fast_flag() {
        let cmd = parse(&["-w", "-f"]);
        assert!(cmd.workspace);
        assert!(cmd.fail_fast);
    }

    #[test]
    fn fail_fast_long() {
        let cmd = parse(&["--workspace", "--fail-fast"]);
        assert!(cmd.workspace);
        assert!(cmd.fail_fast);
    }
}
