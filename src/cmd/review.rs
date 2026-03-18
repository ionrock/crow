use anyhow::{Context, Result};
use owo_colors::OwoColorize;

use crate::gh;
use crate::wt;

pub fn run(pr: u64) -> Result<()> {
    // Gather PR context before switching worktrees
    println!("Fetching PR #{} details...", pr);

    let detail = gh::pr_view(pr)?;
    let diff = gh::pr_diff(pr)?;

    let repo = gh::repo_info()?;
    let threads = gh::review_threads(repo.owner_login(), &repo.name, pr)?;

    let prompt = build_prompt(&detail, &diff, &threads);

    println!(
        "Launching Claude review session for {} by @{}...\n",
        format!("PR #{}", detail.number).cyan(),
        detail.author.login.cyan()
    );

    // Exec into worktree with claude session — does not return on success
    wt::checkout_pr_exec(pr, "claude", &["--dangerously-skip-permissions", &prompt])
        .context("Failed to launch review session")
}

fn build_prompt(
    detail: &crate::types::PrDetail,
    diff: &str,
    threads: &[crate::types::ReviewThread],
) -> String {
    let mut prompt = format!(
        "You are reviewing PR #{}: {}\n\
         Author: @{}\n\
         Branch: {} → {}\n\
         URL: {}\n",
        detail.number,
        detail.title,
        detail.author.login,
        detail.head_ref_name,
        detail.base_ref_name,
        detail.url,
    );

    if !detail.body.is_empty() {
        prompt.push_str(&format!("\n## PR Description\n\n{}\n", detail.body));
    }

    // Files changed summary
    prompt.push_str("\n## Files Changed\n\n");
    for f in &detail.files {
        prompt.push_str(&format!(
            "  {} (+{} -{})  \n",
            f.path, f.additions, f.deletions
        ));
    }

    // Existing review threads
    let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();
    if !unresolved.is_empty() {
        prompt.push_str(&format!(
            "\n## Existing Unresolved Review Comments ({})\n\n",
            unresolved.len()
        ));
        for thread in &unresolved {
            let line_label = match (thread.start_line, thread.line) {
                (Some(s), Some(e)) if s != e => format!("L{}-{}", s, e),
                (_, Some(l)) => format!("L{}", l),
                _ => "L?".to_string(),
            };
            prompt.push_str(&format!("### {} {}\n", thread.path, line_label));
            for c in &thread.comments.nodes {
                prompt.push_str(&format!("@{}: {}\n", c.author.login, c.body));
            }
            prompt.push('\n');
        }
    }

    // Diff
    // Truncate very large diffs to avoid exceeding prompt limits
    let max_diff_len = 100_000;
    let truncated_diff = if diff.len() > max_diff_len {
        let truncated = &diff[..max_diff_len];
        format!(
            "{}\n\n... [diff truncated — {} bytes total, showing first {}]\n",
            truncated,
            diff.len(),
            max_diff_len
        )
    } else {
        diff.to_string()
    };

    prompt.push_str(&format!("\n## Diff\n\n```diff\n{}\n```\n", truncated_diff));

    // Review instructions
    prompt.push_str(
        "\n## Your Task\n\n\
         Review this PR thoroughly. You are in the PR's worktree and can read any file, \
         run tests, or run CI commands.\n\n\
         Focus on:\n\
         - Correctness: bugs, logic errors, edge cases\n\
         - Design: architecture, abstractions, API surface\n\
         - Safety: error handling, security, resource management\n\
         - Style: naming, clarity, idiomatic patterns for this codebase\n\n\
         For each issue found, specify the file and line. \
         Distinguish between must-fix issues and suggestions.\n\n\
         You can run tests (`make test`, `cargo test`, etc.) or explore the code \
         to verify your findings. The user can interact with you to discuss, \
         run additional checks, or make changes.\n",
    );

    prompt
}
