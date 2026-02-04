use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

use tspec::cli::{Cli, Commands, TsCommands};
use tspec::cmd::CargoPassthrough;
use tspec::compare::compare_specs;
use tspec::find_paths::{find_package_dir, find_project_root, find_tspecs};
use tspec::ts_cmd;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(cmd) => {
            return cmd.execute(&find_project_root()?);
        }
        Commands::Run(cmd) => {
            return cmd.execute(&find_project_root()?);
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
        Commands::Install(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
