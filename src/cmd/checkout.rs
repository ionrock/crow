use anyhow::{Context, Result};

use crate::cmd::reviews;
use crate::gh::GhClient;
use crate::wt::WtClient;

pub fn run(gh: &dyn GhClient, wt: &dyn WtClient, pr: u64) -> Result<()> {
    wt.checkout_pr(pr)
        .context("Failed to check out PR worktree")?;

    println!("Checked out PR #{} into worktree.\n", pr);

    // Auto-show unresolved review threads
    reviews::run(gh, Some(pr), false, false, true)
}
