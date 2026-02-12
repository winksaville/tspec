use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, current_package_name};
use crate::compare::compare_specs;
use crate::find_paths::{find_tspecs, get_package_name, resolve_package_dir};

/// Compare specs for a package (size only)
#[derive(Args)]
pub struct CompareCmd {
    /// Package to compare (defaults to current directory)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Spec file(s) or glob pattern(s) (defaults to tspec* pattern)
    #[arg(short = 't', long = "tspec", num_args = 1..)]
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
        let pkg_name = self
            .package
            .clone()
            .or_else(current_package_name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "not in a package directory (no Cargo.toml found). Use -p to specify a package."
                )
            })?;
        let package_dir = resolve_package_dir(project_root, Some(&pkg_name))?;
        let pkg_name = get_package_name(&package_dir)?;
        let spec_paths = find_tspecs(&package_dir, &self.tspec)?;
        compare_specs(&pkg_name, &spec_paths, self.release, self.strip)?;
        Ok(ExitCode::SUCCESS)
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
    fn tspec_with_other_flags() {
        // Shell-expanded glob followed by other flags
        let cmd = parse(&["-t", "a.ts.toml", "b.ts.toml", "-r", "-s"]);
        assert_eq!(cmd.tspec, vec!["a.ts.toml", "b.ts.toml"]);
        assert!(cmd.release);
        assert!(cmd.strip);
    }

    #[test]
    fn all_flags_together() {
        let cmd = parse(&["-p", "myapp", "-t", "spec.ts.toml", "-r", "-s"]);
        assert_eq!(cmd.package.as_deref(), Some("myapp"));
        assert_eq!(cmd.tspec, vec!["spec.ts.toml"]);
        assert!(cmd.release);
        assert!(cmd.strip);
    }
}
