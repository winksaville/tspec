use anyhow::{Context, Result};
use clap::Parser;
use std::process::ExitCode;

use tspec::all::{build_all, print_run_summary, print_summary, run_all};
use tspec::binary::strip_binary;
use tspec::cargo_build::build_crate;
use tspec::cli::{Cli, Commands, TsCommands};
use tspec::cmd::CargoPassthrough;
use tspec::compare::compare_specs;
use tspec::find_paths::{find_package_dir, find_project_root, find_tspecs, get_crate_name};
use tspec::run::run_binary;
use tspec::ts_cmd;
use tspec::workspace::WorkspaceInfo;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Check if current directory is a package (has Cargo.toml with [package])
fn current_package_name() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    get_crate_name(&cwd).ok()
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            package,
            all,
            tspec,
            release,
            strip,
            fail_fast,
        } => {
            // Resolve package: --all > -p PKG > cwd > all
            let resolved = if all {
                None
            } else {
                package.or_else(current_package_name)
            };
            match resolved {
                None => {
                    // Build all packages
                    let workspace = WorkspaceInfo::discover()?;
                    let results =
                        build_all(&workspace, tspec.as_deref(), release, strip, fail_fast);
                    return Ok(print_summary(&results));
                }
                Some(name) => {
                    let result = build_crate(&name, tspec.as_deref(), release)?;
                    if strip {
                        strip_binary(&result.binary_path)?;
                    }
                }
            }
        }
        Commands::Run {
            package,
            all,
            tspec,
            release,
            strip,
            args,
        } => {
            // Resolve package: --all > -p PKG > cwd > all
            let resolved = if all {
                None
            } else {
                package.or_else(current_package_name)
            };
            match resolved {
                None => {
                    // Run all apps (args not supported for --all)
                    let workspace = WorkspaceInfo::discover()?;
                    let results = run_all(&workspace, tspec.as_deref(), release, strip);
                    return Ok(print_run_summary(&results));
                }
                Some(name) => {
                    // Build, optionally strip, then run
                    let result = build_crate(&name, tspec.as_deref(), release)?;
                    if strip {
                        strip_binary(&result.binary_path)?;
                    }
                    let exit_code = run_binary(&result.binary_path, &args)?;
                    std::process::exit(exit_code);
                }
            }
        }
        Commands::Test(cmd) => {
            return cmd.execute(&find_project_root()?);
        }
        Commands::Clean(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
        Commands::Clippy(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
        Commands::Fmt(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
        Commands::Compare {
            package,
            tspec,
            release,
            strip,
        } => {
            let workspace = find_project_root()?;
            let package_dir = find_package_dir(&workspace, &package)?;
            let spec_paths = find_tspecs(&package_dir, &tspec)?;
            compare_specs(&package, &spec_paths, release, strip)?;
        }
        Commands::Ts { command } => match command {
            TsCommands::List { package, all } => {
                ts_cmd::list_tspecs(package.as_deref(), all)?;
            }
            TsCommands::Show {
                package,
                all,
                tspec,
            } => {
                ts_cmd::show_tspec(package.as_deref(), all, tspec.as_deref())?;
            }
            TsCommands::Hash {
                package,
                all,
                tspec,
            } => {
                ts_cmd::hash_tspec(package.as_deref(), all, tspec.as_deref())?;
            }
            TsCommands::New {
                name,
                package,
                from,
            } => {
                ts_cmd::new_tspec(package.as_deref(), &name, from.as_deref())?;
            }
            TsCommands::Set {
                assignment,
                package,
                tspec,
            } => {
                let (key, value) = assignment.split_once('=').ok_or_else(|| {
                    anyhow::anyhow!("invalid assignment '{}': expected key=value", assignment)
                })?;
                ts_cmd::set_value(package.as_deref(), key, value, tspec.as_deref())?;
            }
        },
        Commands::Version => {
            println!("tspec {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::Install { path, force } => {
            let resolved = path
                .canonicalize()
                .with_context(|| format!("path not found: {}", path.display()))?;
            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("install").arg("--path").arg(&resolved);
            if force {
                cmd.arg("--force");
            }
            let status = cmd.status().context("failed to run cargo install")?;
            if !status.success() {
                anyhow::bail!("cargo install failed");
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
