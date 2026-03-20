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
        Command::Checkout { pr } => cmd::checkout::run(&gh, &wt, pr),
        Command::Reviews {
            pr,
            all,
            diff,
            unresolved,
        } => cmd::reviews::run(&gh, pr, all, diff, unresolved),
        Command::Ci { pr, watch } => cmd::ci::run(&gh, pr, watch),
        Command::Push { reply } => cmd::push::run(&gh, reply),
        Command::Done { ready } => cmd::done::run(&gh, &wt, ready),
        Command::Review { pr } => cmd::review::run(&gh, pr),
        Command::InstallPlugin { uninstall } => cmd::install_plugin::run(uninstall),
        Command::Comment { pr, event, body } => cmd::comment::run(&gh, pr, event, body),
    }
}
