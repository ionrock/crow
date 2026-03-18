use anyhow::{Context, Result};

use crate::cmd::reviews;
use crate::wt;

pub fn run(pr: u64) -> Result<()> {
    wt::checkout_pr(pr).context("Failed to check out PR worktree")?;

    println!("Checked out PR #{} into worktree.\n", pr);

    // Auto-show unresolved review threads
    reviews::run(Some(pr), false, false, true)
}
