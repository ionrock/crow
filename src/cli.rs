use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "crow", about = "Code Review Workflow Accelerator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show PRs needing attention
    Status,

    /// Review a PR with Claude in a worktree
    Review {
        /// PR number to review
        pr: u64,
    },

    /// Install (or uninstall) the crow Claude Code plugin
    InstallPlugin {
        /// Remove the plugin instead of installing
        #[arg(long)]
        uninstall: bool,
    },
}
