use clap::Parser;
use owo_colors::OwoColorize;

mod cli;
mod cmd;
mod display;
mod gh;
#[cfg(test)]
mod test_helpers;
mod types;
mod wt;

use cli::{Cli, Command};
use gh::RealGhClient;
use wt::RealWtClient;

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{} {:#}", "error:".red().bold(), err);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let gh = RealGhClient;
    let wt = RealWtClient;

    match cli.command {
        Command::Status => cmd::status::run(&gh),
        Command::Review { pr } => cmd::review::run(&gh, &wt, pr),
        Command::InstallPlugin { uninstall } => cmd::install_plugin::run(uninstall),
    }
}
