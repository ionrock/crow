use clap::{Parser, Subcommand, ValueEnum};

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

    /// Check out a PR branch into a worktree
    Checkout {
        /// PR number to check out
        pr: u64,
    },

    /// Show review comments grouped by file
    Reviews {
        /// PR number (defaults to current branch)
        pr: Option<u64>,

        /// Include resolved threads
        #[arg(long)]
        all: bool,

        /// Show diff hunks
        #[arg(long)]
        diff: bool,

        /// Show only unresolved threads (default: true)
        #[arg(long, default_value_t = true)]
        unresolved: bool,
    },

    /// Show CI check status
    Ci {
        /// PR number (defaults to current branch)
        pr: Option<u64>,

        /// Watch checks until completion
        #[arg(long)]
        watch: bool,
    },

    /// Push changes on the current PR branch
    Push {
        /// Batch-reply to all unresolved threads with this message
        #[arg(long)]
        reply: Option<String>,
    },

    /// Clean up after finishing work on a PR
    Done {
        /// Mark own PR as ready for re-review
        #[arg(long)]
        ready: bool,
    },

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

    /// Post a review on a PR
    Comment {
        /// PR number
        pr: u64,

        /// Review event type
        #[arg(long, value_enum, default_value_t = ReviewEvent::Comment)]
        event: ReviewEvent,

        /// Review body (opened in $EDITOR if omitted)
        body: Option<String>,
    },
}

#[derive(ValueEnum, Debug, Clone)]
pub enum ReviewEvent {
    /// Approve the PR
    Approve,
    /// Request changes on the PR
    RequestChanges,
    /// Leave a comment without approving or requesting changes
    Comment,
}
