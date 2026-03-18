use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::process::Command;

use crate::gh;
use crate::wt;

pub fn run(ready: bool) -> Result<()> {
    let pr = gh::current_pr_number()?;
    let pr_author = gh::pr_author(pr)?;
    let current_user = gh::current_user()?;
    let is_own_pr = pr_author == current_user;

    if is_own_pr {
        // Push any remaining changes (ignore "nothing to push" errors)
        let output = Command::new("git")
            .args(["push"])
            .output()
            .context("Failed to run git push")?;

        if output.status.success() {
            println!("{}", "Pushed remaining changes.".green());
        }

        if ready {
            gh::mark_ready(pr)?;
            println!("Marked PR #{} as {}.", pr, "ready for review".green());
        }
    }

    wt::remove_current()?;
    println!("Removed worktree for PR #{}.", pr);

    Ok(())
}
