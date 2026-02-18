use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::process::ExitCode;

use super::{Execute, current_package_name, resolve_package_arg};
use crate::all::{print_run_summary, run_all};
use crate::binary::strip_binary;
use crate::cargo_build::build_package;
use crate::find_paths::{find_tspecs, get_package_name, resolve_package_dir};
use crate::run::run_binary;
use crate::workspace::WorkspaceInfo;

/// Build and run package(s) with a translation spec
#[derive(Args)]
pub struct RunCmd {
    /// Package to run (name or path, e.g. "." for current dir)
    #[arg(value_name = "PACKAGE")]
    pub positional: Option<String>,
    /// Package to run (defaults to current directory or all apps)
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,
    /// Run all workspace apps (even when in a package directory)
    #[arg(short = 'w', long = "workspace")]
    pub workspace: bool,
    /// Spec file(s) or glob pattern(s) to run with (defaults to package's tspec file)
    #[arg(short = 't', long = "tspec", num_args = 1..)]
    pub tspec: Vec<String>,
    /// Release build
    #[arg(short, long, conflicts_with = "profile")]
    pub release: bool,
    /// Build profile (e.g., release, release-small, or any custom profile)
    #[arg(long)]
    pub profile: Option<String>,
    /// Strip symbols from binary before running
    #[arg(short, long)]
    pub strip: bool,
    /// Arguments to pass to the binary (after --)
    #[arg(last = true)]
    pub args: Vec<String>,
}

impl RunCmd {
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

impl Execute for RunCmd {
    fn execute(&self, project_root: &Path) -> Result<ExitCode> {
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
                // Run all apps (args not supported for --workspace)
                let workspace = WorkspaceInfo::discover()?;
                let results = run_all(&workspace, &self.tspec, cli_profile, self.strip);
                Ok(print_run_summary(workspace.name(), &results))
            }
            Some(name) => {
                if self.tspec.is_empty() {
                    // Build, optionally strip, then run
                    let result = build_package(&name, None, cli_profile)?;
                    if self.strip {
                        strip_binary(&result.binary_path)?;
                    }
                    let exit_code = run_binary(&result.binary_path, &self.args)?;
                    std::process::exit(exit_code);
                } else {
                    let package_dir = resolve_package_dir(project_root, Some(&name))?;
                    let pkg_name = get_package_name(&package_dir)?;
                    let spec_paths = find_tspecs(&package_dir, &self.tspec)?;
                    for spec_path in &spec_paths {
                        let spec_str = spec_path.to_string_lossy();
                        let result = build_package(&pkg_name, Some(&spec_str), cli_profile)?;
                        if self.strip {
                            strip_binary(&result.binary_path)?;
                        }
                        let exit_code = run_binary(&result.binary_path, &self.args)?;
                        if exit_code != 0 {
                            std::process::exit(exit_code);
                        }
                    }
                    Ok(ExitCode::SUCCESS)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    fn parse(args: &[&str]) -> RunCmd {
        let mut full = vec!["tspec", "run"];
        full.extend_from_slice(args);
        let cli = Cli::try_parse_from(full).unwrap();
        match cli.command {
            crate::cli::Commands::Run(cmd) => cmd,
            _ => panic!("expected Run command"),
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
    fn args_after_double_dash() {
        let cmd = parse(&["-p", "myapp", "--", "arg1", "arg2"]);
        assert_eq!(cmd.args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn profile_flag() {
        let cmd = parse(&["--profile", "release-small"]);
        assert_eq!(cmd.profile.as_deref(), Some("release-small"));
        assert_eq!(cmd.effective_profile(), Some("release-small"));
    }

    #[test]
    fn profile_and_release_conflict() {
        let result = Cli::try_parse_from(["tspec", "run", "-r", "--profile", "custom"]);
        assert!(result.is_err());
    }
}
