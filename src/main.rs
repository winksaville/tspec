use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

use tspec::cli::{Cli, Commands};
use tspec::cmd::Execute;
use tspec::find_paths::find_project_root;

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
