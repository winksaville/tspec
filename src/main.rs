use clap::Parser;
use std::process::ExitCode;

use tspec::cli::{Cli, Commands};
use tspec::cmd::Execute;
use tspec::find_paths::find_project_root;
use tspec::types::Verbosity;

fn main() -> Result<ExitCode, anyhow::Error> {
    let cli = Cli::parse();
    let verbosity = Verbosity::from_count(cli.verbose);

    match cli.command {
        Commands::Build(cmd) => {
            return cmd.execute(&find_project_root()?, verbosity);
        }
        Commands::Run(cmd) => {
            return cmd.execute(&find_project_root()?, verbosity);
        }
        Commands::Test(cmd) => {
            return cmd.execute(&find_project_root()?, verbosity);
        }
        Commands::Clean(cmd) => {
            cmd.execute(&find_project_root()?, verbosity)?;
        }
        Commands::Clippy(cmd) => {
            cmd.execute(&find_project_root()?, verbosity)?;
        }
        Commands::Fmt(cmd) => {
            cmd.execute(&find_project_root()?, verbosity)?;
        }
        Commands::Compare(cmd) => {
            cmd.execute(&find_project_root()?, verbosity)?;
        }
        Commands::Ts(cmd) => {
            cmd.execute(&find_project_root()?, verbosity)?;
        }
        Commands::Version(cmd) => {
            cmd.execute(&find_project_root()?, verbosity)?;
        }
        Commands::Install(cmd) => {
            cmd.execute(&find_project_root()?, verbosity)?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
