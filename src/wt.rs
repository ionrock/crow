// wt.rs — adapter for all `wt` command wrappers

use anyhow::{Context, Result};
use std::process::Command;

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

// ---------------------------------------------------------------------------
// WtClient trait
// ---------------------------------------------------------------------------

pub trait WtClient {
    fn checkout_pr(&self, pr: u64) -> Result<()>;
    /// Check out a PR into a worktree and exec into a command, replacing the
    /// current process. Only returns on error.
    fn checkout_pr_exec(&self, pr: u64, cmd: &str, args: &[&str]) -> Result<()>;
    fn remove_current(&self) -> Result<()>;
}

// ---------------------------------------------------------------------------
// RealWtClient — production implementation backed by the `wt` CLI
// ---------------------------------------------------------------------------

pub struct RealWtClient;

impl WtClient for RealWtClient {
    fn checkout_pr(&self, pr: u64) -> Result<()> {
        let target = format!("pr:{}", pr);
        run_wt(&["switch", &target]).context("Failed to switch to PR worktree")
    }

    fn checkout_pr_exec(&self, pr: u64, cmd: &str, args: &[&str]) -> Result<()> {
        use std::os::unix::process::CommandExt;

        let target = format!("pr:{}", pr);
        let err = Command::new("wt")
            .args(["switch", &target, "--execute", cmd, "--"])
            .args(args)
            .exec();

        // exec() only returns on error
        anyhow::bail!("Failed to exec into worktree: {}", err)
    }

    fn remove_current(&self) -> Result<()> {
        run_wt(&["remove"]).context("Failed to remove current worktree")
    }
}
