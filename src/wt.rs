// wt.rs — adapter for all `wt` command wrappers

use anyhow::{Context, Result};
use std::process::Command;

// ---------------------------------------------------------------------------
// WtClient trait — injectable for testing
// ---------------------------------------------------------------------------

pub trait WtClient {
    fn checkout_pr(&self, pr: u64) -> Result<()>;
    fn remove_current(&self) -> Result<()>;
}

// ---------------------------------------------------------------------------
// Real implementation
// ---------------------------------------------------------------------------

pub struct RealWtClient;

impl WtClient for RealWtClient {
    fn checkout_pr(&self, pr: u64) -> Result<()> {
        checkout_pr(pr)
    }
    fn remove_current(&self) -> Result<()> {
        remove_current()
    }
}

// ---------------------------------------------------------------------------
// Free functions (used by RealWtClient and directly by review.rs)
// ---------------------------------------------------------------------------

fn run_wt(args: &[&str]) -> Result<()> {
    let output = Command::new("wt")
        .args(args)
        .output()
        .context("Failed to run wt — is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wt command failed: {}", stderr.trim());
    }

    Ok(())
}

pub fn checkout_pr(pr: u64) -> Result<()> {
    let target = format!("pr:{}", pr);
    run_wt(&["switch", &target]).context("Failed to switch to PR worktree")
}

/// Check out a PR into a worktree and exec into a command, replacing the
/// current process. Only returns on error.
pub fn checkout_pr_exec(pr: u64, cmd: &str, args: &[&str]) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let target = format!("pr:{}", pr);
    let err = Command::new("wt")
        .args(["switch", &target, "--execute", cmd, "--"])
        .args(args)
        .exec();

    // exec() only returns on error
    anyhow::bail!("Failed to exec into worktree: {}", err)
}

pub fn remove_current() -> Result<()> {
    run_wt(&["remove"]).context("Failed to remove current worktree")
}
