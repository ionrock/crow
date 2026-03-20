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

pub(crate) fn build_prompt(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Author, PrDetail, PrFile, ReviewThread, ThreadComment, ThreadComments};

    fn make_detail(body: &str, files: Vec<PrFile>) -> PrDetail {
        PrDetail {
            number: 1,
            title: "Test PR".to_string(),
            body: body.to_string(),
            head_ref_name: "feat/test".to_string(),
            base_ref_name: "main".to_string(),
            author: Author {
                login: "tester".to_string(),
            },
            url: "https://github.com/owner/repo/pull/1".to_string(),
            files,
        }
    }

    fn make_thread(
        id: &str,
        is_resolved: bool,
        path: &str,
        line: Option<u64>,
        start_line: Option<u64>,
        comments: Vec<ThreadComment>,
    ) -> ReviewThread {
        ReviewThread {
            id: id.to_string(),
            is_resolved,
            is_outdated: false,
            path: path.to_string(),
            line,
            start_line,
            comments: ThreadComments { nodes: comments },
        }
    }

    fn make_comment(login: &str, body: &str) -> ThreadComment {
        ThreadComment {
            id: "c1".to_string(),
            author: Author {
                login: login.to_string(),
            },
            body: body.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            url: "https://github.com/owner/repo/pull/1#comment-c1".to_string(),
            diff_hunk: "@@ -1,3 +1,4 @@".to_string(),
        }
    }

    #[test]
    fn test_build_prompt_no_body_no_threads() {
        let detail = make_detail("", vec![]);
        let prompt = build_prompt(&detail, "--- a/foo\n+++ b/foo\n+hello", &[]);

        assert!(prompt.contains("PR #1"));
        assert!(prompt.contains("Test PR"));
        assert!(prompt.contains("@tester"));
        assert!(prompt.contains("feat/test"));
        assert!(prompt.contains("main"));
        assert!(prompt.contains("## Files Changed"));
        assert!(prompt.contains("## Diff"));
        assert!(prompt.contains("## Your Task"));
        // No description section when body is empty
        assert!(!prompt.contains("## PR Description"));
    }

    #[test]
    fn test_build_prompt_with_body() {
        let detail = make_detail("This PR fixes a nasty bug.", vec![]);
        let prompt = build_prompt(&detail, "", &[]);

        assert!(prompt.contains("## PR Description"));
        assert!(prompt.contains("This PR fixes a nasty bug."));
    }

    #[test]
    fn test_build_prompt_with_unresolved_threads() {
        let comment = make_comment("reviewer", "Please rename this variable.");
        let thread = make_thread("t1", false, "src/lib.rs", Some(42), None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("## Existing Unresolved Review Comments (1)"));
        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("L42"));
        assert!(prompt.contains("@reviewer"));
        assert!(prompt.contains("Please rename this variable."));
    }

    #[test]
    fn test_build_prompt_only_resolved_threads_no_section() {
        let comment = make_comment("reviewer", "Fixed already.");
        let thread = make_thread("t2", true, "src/lib.rs", Some(10), None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_prompt(&detail, "", &[thread]);

        assert!(!prompt.contains("## Existing Unresolved Review Comments"));
    }

    #[test]
    fn test_build_prompt_large_diff_truncation() {
        let large_diff = "x".repeat(200_000);
        let detail = make_detail("", vec![]);
        let prompt = build_prompt(&detail, &large_diff, &[]);

        assert!(prompt.contains("diff truncated"));
        assert!(prompt.contains("200000 bytes total"));
        assert!(prompt.contains("showing first 100000"));
    }

    #[test]
    fn test_build_prompt_files_changed_section() {
        let files = vec![
            PrFile {
                path: "src/main.rs".to_string(),
                additions: 15,
                deletions: 3,
            },
            PrFile {
                path: "Cargo.toml".to_string(),
                additions: 2,
                deletions: 0,
            },
        ];
        let detail = make_detail("", files);
        let prompt = build_prompt(&detail, "", &[]);

        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("+15"));
        assert!(prompt.contains("-3"));
        assert!(prompt.contains("Cargo.toml"));
        assert!(prompt.contains("+2"));
        assert!(prompt.contains("-0"));
    }

    #[test]
    fn test_build_prompt_thread_line_range() {
        let comment = make_comment("rev", "Range comment");
        let thread = make_thread("t3", false, "src/foo.rs", Some(20), Some(15), vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("L15-20"));
    }

    #[test]
    fn test_build_prompt_thread_no_line_number() {
        let comment = make_comment("rev", "No line comment");
        let thread = make_thread("t4", false, "src/bar.rs", None, None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("L?"));
    }
}
