use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, current_package_name, resolve_package_arg};
use crate::all::{build_all, print_summary};
use crate::binary::strip_binary;
use crate::cargo_build::build_package;
use crate::find_paths::{find_tspecs, get_package_name, resolve_package_dir};
use crate::types::Verbosity;
use crate::workspace::WorkspaceInfo;

/// Build package(s) with a translation spec
#[derive(Args)]
pub struct BuildCmd {
    /// Package to build (name or path, e.g. "." for current dir)
    #[arg(value_name = "PACKAGE")]
    pub positional: Option<String>,
    /// Package to build (defaults to current directory or all packages)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Build all workspace packages (even when in a package directory)
    #[arg(short = 'w', long = "workspace")]
    pub workspace: bool,
    /// Spec file(s) or glob pattern(s) to build with (defaults to package's tspec file)
    #[arg(short = 't', long = "tspec", num_args = 1..)]
    pub tspec: Vec<String>,
    /// Release build
    #[arg(short, long, conflicts_with = "profile")]
    pub release: bool,
    /// Build profile (e.g., release, release-small, or any custom profile)
    #[arg(long)]
    pub profile: Option<String>,
    /// Strip symbols from binary after build
    #[arg(short, long)]
    pub strip: bool,
    /// Stop on first failure (for all-packages mode)
    #[arg(short, long)]
    pub fail_fast: bool,
}

impl BuildCmd {
    /// Resolve the effective profile from --release / --profile flags.
    fn effective_profile(&self) -> Option<&str> {
        if let Some(ref p) = self.profile {
            Some(p)
        } else if self.release {
            Some("release")
        } else {
            None
        }
    }
}

impl Execute for BuildCmd {
    fn execute(&self, project_root: &Path, verbosity: Verbosity) -> Result<ExitCode> {
        let cli_profile = self.effective_profile();

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
                let results = build_all(
                    &workspace,
                    &self.tspec,
                    cli_profile,
                    self.strip,
                    self.fail_fast,
                    verbosity,
                );
                Ok(print_summary(workspace.name(), &results))
            }
            Some(name) => {
                if self.tspec.is_empty() {
                    let result = build_package(&name, None, cli_profile, verbosity)?;
                    if self.strip {
                        strip_binary(&result.binary_path)?;
                    }
                } else {
                    let package_dir = resolve_package_dir(project_root, Some(&name))?;
                    let pkg_name = get_package_name(&package_dir)?;
                    let spec_paths = find_tspecs(&package_dir, &self.tspec)?;
                    for spec_path in &spec_paths {
                        let spec_str = spec_path.to_string_lossy();
                        let result =
                            build_package(&pkg_name, Some(&spec_str), cli_profile, verbosity)?;
                        if self.strip {
                            strip_binary(&result.binary_path)?;
                        }
                    }
                }
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

    fn parse(args: &[&str]) -> BuildCmd {
        let mut full = vec!["tspec", "build"];
        full.extend_from_slice(args);
        let cli = Cli::try_parse_from(full).unwrap();
        match cli.command {
            crate::cli::Commands::Build(cmd) => cmd,
            _ => panic!("expected Build command"),
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

    #[test]
    fn release_flag() {
        let cmd = parse(&["-r"]);
        assert!(cmd.release);
    }

    #[test]
    fn strip_flag() {
        let cmd = parse(&["-s"]);
        assert!(cmd.strip);
    }

    #[test]
    fn profile_flag() {
        let cmd = parse(&["--profile", "release-small"]);
        assert_eq!(cmd.profile.as_deref(), Some("release-small"));
        assert_eq!(cmd.effective_profile(), Some("release-small"));
    }

    #[test]
    fn release_effective_profile() {
        let cmd = parse(&["-r"]);
        assert_eq!(cmd.effective_profile(), Some("release"));
    }

    #[test]
    fn no_profile_effective() {
        let cmd = parse(&[]);
        assert_eq!(cmd.effective_profile(), None);
    }

    #[test]
    fn profile_and_release_conflict() {
        let result = Cli::try_parse_from(["tspec", "build", "-r", "--profile", "custom"]);
        assert!(result.is_err());
    }
}
