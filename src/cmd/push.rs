use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::process::Command;

use crate::gh;

pub fn run(reply: Option<String>) -> Result<()> {
    // Run git push
    let output = Command::new("git")
        .args(["push"])
        .output()
        .context("Failed to run git push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git push failed: {}", stderr.trim());
    }

    println!("{}", "Pushed.".green());

    // If --reply provided, batch-reply to all unresolved threads
    if let Some(msg) = reply {
        let repo = gh::repo_info()?;
        let pr = gh::current_pr_number()?;
        let threads = gh::review_threads(repo.owner_login(), &repo.name, pr)?;

        let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();

        if unresolved.is_empty() {
            println!("No unresolved threads to reply to.");
            return Ok(());
        }

        let mut replied = 0;
        for thread in &unresolved {
            // Reply to the last comment in each thread
            if let Some(last_comment) = thread.comments.nodes.last() {
                gh::reply_to_thread(repo.owner_login(), &repo.name, pr, &last_comment.id, &msg)?;
                replied += 1;
            }
        }

        println!(
            "Replied \"{}\" to {} unresolved thread{}.",
            msg,
            replied,
            if replied == 1 { "" } else { "s" }
        );
    }

    Ok(())
}
