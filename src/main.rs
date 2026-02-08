use clap::Parser;
use std::process::ExitCode;

use tspec::cli::{Cli, Commands};
use tspec::cmd::Execute;
use tspec::find_paths::find_project_root;

fn main() -> Result<ExitCode, anyhow::Error> {
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
        Commands::Compare(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
        Commands::Ts(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
        Commands::Version(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
        Commands::Install(cmd) => {
            cmd.execute(&find_project_root()?)?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
