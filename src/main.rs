use clap::Parser;
use owo_colors::OwoColorize;

mod cli;
mod cmd;
mod display;
mod gh;
mod types;
mod wt;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{} {:#}", "error:".red().bold(), err);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Status => cmd::status::run(),
        Command::Checkout { pr } => cmd::checkout::run(pr),
        Command::Reviews {
            pr,
            all,
            diff,
            unresolved,
        } => cmd::reviews::run(pr, all, diff, unresolved),
        Command::Ci { pr, watch } => cmd::ci::run(pr, watch),
        Command::Push { reply } => cmd::push::run(reply),
        Command::Done { ready } => cmd::done::run(ready),
        Command::Comment { pr, event, body } => cmd::comment::run(pr, event, body),
    }
}
