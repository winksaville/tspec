use clap::Parser;
use std::process::ExitCode;

use tspec::cli::{Cli, Commands};
use tspec::cmd::Execute;
use tspec::find_paths::find_project_root;
use tspec::types::{CargoFlags, Verbosity};

fn main() -> Result<ExitCode, anyhow::Error> {
    let cli = Cli::parse();
    let flags = CargoFlags {
        verbosity: Verbosity::from_count(cli.verbose),
        jobs: cli.jobs,
        extra_args: Vec::new(),
    };

    match cli.command {
        Commands::Build(cmd) => {
            return cmd.execute(&find_project_root()?, &flags);
        }
        Commands::Run(cmd) => {
            return cmd.execute(&find_project_root()?, &flags);
        }
        Commands::Test(cmd) => {
            return cmd.execute(&find_project_root()?, &flags);
        }
        Commands::Clean(cmd) => {
            cmd.execute(&find_project_root()?, &flags)?;
        }
        Commands::Clippy(cmd) => {
            cmd.execute(&find_project_root()?, &flags)?;
        }
        Commands::Fmt(cmd) => {
            cmd.execute(&find_project_root()?, &flags)?;
        }
        Commands::Compare(cmd) => {
            cmd.execute(&find_project_root()?, &flags)?;
        }
        Commands::Ts(cmd) => {
            cmd.execute(&find_project_root()?, &flags)?;
        }
        Commands::Version(cmd) => {
            cmd.execute(&find_project_root()?, &flags)?;
        }
        Commands::Install(cmd) => {
            cmd.execute(&find_project_root()?, &flags)?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
