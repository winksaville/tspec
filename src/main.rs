use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

use tspec::cli::{Cli, Commands};
use tspec::cmd::Execute;
use tspec::find_paths::find_project_root;

fn main() -> Result<ExitCode, anyhow::Error> {
    println!("args: {:?}", std::env::args().collect::<Vec<_>>());
    let exe_path: std::path::PathBuf = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => return Err(anyhow::anyhow!(e)),
    };
    println!("Executable path: {}", exe_path.display());
    run()
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
