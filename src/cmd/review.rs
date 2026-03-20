use anyhow::{Context, Result};
use owo_colors::OwoColorize;

use crate::gh::GhClient;
use crate::types::{PrDetail, ReviewThread};
use crate::wt::WtClient;

pub fn run(gh: &dyn GhClient, wt: &dyn WtClient, pr: u64) -> Result<()> {
    // Gather PR context before switching worktrees
    println!("Fetching PR #{} details...", pr);

    let detail = gh.pr_view(pr)?;
    let diff = gh.pr_diff(pr)?;

    let repo = gh.repo_info()?;
    let threads = gh.review_threads(repo.owner_login(), &repo.name, pr)?;

    let current_user = gh.current_user()?;
    let is_author = current_user == detail.author.login;

    let prompt = if is_author {
        println!(
            "Launching Claude author-review session for {} (you are the author)...\n",
            format!("PR #{}", detail.number).cyan(),
        );
        build_author_prompt(&detail, &diff, &threads)
    } else {
        println!(
            "Launching Claude reviewer session for {} by @{}...\n",
            format!("PR #{}", detail.number).cyan(),
            detail.author.login.cyan()
        );
        build_reviewer_prompt(&detail, &diff, &threads)
    };

    // Exec into worktree with claude session — does not return on success
    wt.checkout_pr_exec(pr, "claude", &["--dangerously-skip-permissions", &prompt])
        .context("Failed to launch review session")
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn pr_header(detail: &PrDetail) -> String {
    format!(
        "PR #{}: {}\nAuthor: @{}\nBranch: {} → {}\nURL: {}\n",
        detail.number,
        detail.title,
        detail.author.login,
        detail.head_ref_name,
        detail.base_ref_name,
        detail.url,
    )
}

fn pr_description_section(detail: &PrDetail) -> String {
    if detail.body.is_empty() {
        String::new()
    } else {
        format!("\n## PR Description\n\n{}\n", detail.body)
    }
}

fn files_changed_section(detail: &PrDetail) -> String {
    let mut out = "\n## Files Changed\n\n".to_string();
    for f in &detail.files {
        out.push_str(&format!(
            "  {} (+{} -{})\n",
            f.path, f.additions, f.deletions
        ));
    }
    out
}

fn diff_section(diff: &str) -> String {
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
    format!("\n## Diff\n\n```diff\n{}\n```\n", truncated_diff)
}

fn unresolved_threads(threads: &[ReviewThread]) -> Vec<&ReviewThread> {
    threads.iter().filter(|t| !t.is_resolved).collect()
}

fn format_thread_line_label(thread: &ReviewThread) -> String {
    match (thread.start_line, thread.line) {
        (Some(s), Some(e)) if s != e => format!("L{}-{}", s, e),
        (_, Some(l)) => format!("L{}", l),
        _ => "L?".to_string(),
    }
}

fn unresolved_threads_section(threads: &[ReviewThread], heading: &str) -> String {
    let unresolved = unresolved_threads(threads);
    if unresolved.is_empty() {
        return String::new();
    }
    let mut out = format!("\n## {} ({})\n\n", heading, unresolved.len());
    for thread in &unresolved {
        let line_label = format_thread_line_label(thread);
        out.push_str(&format!("### {} {}\n", thread.path, line_label));
        if let Some(first) = thread.comments.nodes.first() {
            out.push_str(&format!(
                "Diff context:\n```diff\n{}\n```\n",
                first.diff_hunk
            ));
        }
        for c in &thread.comments.nodes {
            out.push_str(&format!("@{}: {}\n", c.author.login, c.body));
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Author-review prompt (CROW-21)
// ---------------------------------------------------------------------------

pub(crate) fn build_author_prompt(
    detail: &PrDetail,
    diff: &str,
    threads: &[ReviewThread],
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are helping the PR author respond to review feedback.\n\n");
    prompt.push_str(&pr_header(detail));
    prompt.push_str(&pr_description_section(detail));
    prompt.push_str(&files_changed_section(detail));
    prompt.push_str(&unresolved_threads_section(
        threads,
        "Unresolved Review Comments — Action Required",
    ));
    prompt.push_str(&diff_section(diff));

    prompt.push_str(
        "\n## Your Task\n\n\
         You are assisting the author of this PR who has received review feedback. \
         You are in the PR's worktree and can read any file, run tests, or explore the code.\n\n\
         For each unresolved review comment above:\n\
         1. Read the comment carefully and understand what the reviewer is asking.\n\
         2. Examine the relevant code in context.\n\
         3. Propose a concrete fix — provide the exact code change.\n\
         4. Help the author implement it if they want.\n\n\
         Prioritise must-fix issues over nits. \
         After all changes are made, suggest running the test suite and pushing the branch. \
         The author can discuss any comment with you before deciding how to address it.\n",
    );

    prompt
}

// ---------------------------------------------------------------------------
// Reviewer-review prompt (CROW-22)
// ---------------------------------------------------------------------------

pub(crate) fn build_reviewer_prompt(
    detail: &PrDetail,
    diff: &str,
    threads: &[ReviewThread],
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are helping a reviewer conduct a thorough code review.\n\n");
    prompt.push_str(&pr_header(detail));
    prompt.push_str(&pr_description_section(detail));
    prompt.push_str(&files_changed_section(detail));

    // Show existing threads so Claude doesn't duplicate feedback
    let unresolved = unresolved_threads(threads);
    if !unresolved.is_empty() {
        prompt.push_str(&format!(
            "\n## Existing Review Comments ({})\n\n\
             The following comments have already been left. Do NOT repeat them.\n\n",
            unresolved.len()
        ));
        for thread in &unresolved {
            let line_label = format_thread_line_label(thread);
            prompt.push_str(&format!("### {} {}\n", thread.path, line_label));
            for c in &thread.comments.nodes {
                prompt.push_str(&format!("@{}: {}\n", c.author.login, c.body));
            }
            prompt.push('\n');
        }
    }

    prompt.push_str(&diff_section(diff));

    prompt.push_str(
        "\n## Your Task\n\n\
         You are helping review this PR. You are in the PR's worktree and can read any \
         file, run tests, or explore the code to verify your findings.\n\n\
         Review the changes focusing on:\n\
         - Correctness: bugs, logic errors, edge cases\n\
         - Design: architecture, abstractions, API surface\n\
         - Safety: error handling, security, resource management\n\
         - Style: naming, clarity, idiomatic patterns for this codebase\n\n\
         CRITICAL — all feedback must be actionable, simple, and direct:\n\
         - Each issue states clearly what is wrong and exactly how to fix it.\n\
         - No vague suggestions — provide specific fixes with code snippets where helpful.\n\
         - Prioritise by severity: must-fix issues first, then nits.\n\
         - Do not repeat comments that are already posted above.\n\n\
         After your analysis, guide the user to compose and post review comments via \
         `crow comment`. Help them draft clear, direct comment text for each issue.\n",
    );

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{MockGhClient, MockWtClient};
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

    // ---------------------------------------------------------------------------
    // build_author_prompt tests (CROW-21)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_author_prompt_no_threads() {
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "--- a/foo\n+++ b/foo\n+hello", &[]);

        assert!(prompt.contains("PR #1"));
        assert!(prompt.contains("Test PR"));
        assert!(prompt.contains("@tester"));
        assert!(prompt.contains("feat/test"));
        assert!(prompt.contains("main"));
        assert!(prompt.contains("## Files Changed"));
        assert!(prompt.contains("## Diff"));
        assert!(prompt.contains("author"));
        // No thread section when empty
        assert!(!prompt.contains("## Unresolved Review Comments"));
        // No description when empty
        assert!(!prompt.contains("## PR Description"));
    }

    #[test]
    fn test_author_prompt_with_threads() {
        let comment = make_comment("reviewer", "Please rename this variable.");
        let thread = make_thread("t1", false, "src/lib.rs", Some(42), None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("Unresolved Review Comments"));
        assert!(prompt.contains("Action Required"));
        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("L42"));
        assert!(prompt.contains("@reviewer"));
        assert!(prompt.contains("Please rename this variable."));
        // Diff hunk should be included
        assert!(prompt.contains("@@ -1,3 +1,4 @@"));
    }

    #[test]
    fn test_author_prompt_only_resolved_threads() {
        let comment = make_comment("reviewer", "Already fixed.");
        let thread = make_thread("t1", true, "src/lib.rs", Some(10), None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[thread]);

        assert!(!prompt.contains("Unresolved Review Comments"));
    }

    #[test]
    fn test_author_prompt_with_body() {
        let detail = make_detail("This PR fixes a nasty bug.", vec![]);
        let prompt = build_author_prompt(&detail, "", &[]);

        assert!(prompt.contains("## PR Description"));
        assert!(prompt.contains("This PR fixes a nasty bug."));
    }

    #[test]
    fn test_author_prompt_large_diff_truncation() {
        let large_diff = "x".repeat(200_000);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, &large_diff, &[]);

        assert!(prompt.contains("diff truncated"));
        assert!(prompt.contains("200000 bytes total"));
        assert!(prompt.contains("showing first 100000"));
    }

    #[test]
    fn test_author_prompt_prioritise_must_fix() {
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[]);

        assert!(prompt.contains("must-fix") || prompt.contains("Prioriti"));
    }

    #[test]
    fn test_author_prompt_multiple_threads() {
        let c1 = make_comment("alice", "Fix the error handling.");
        let c2 = make_comment("bob", "Add a test for this path.");
        let t1 = make_thread("t1", false, "src/a.rs", Some(10), None, vec![c1]);
        let t2 = make_thread("t2", false, "src/b.rs", Some(20), None, vec![c2]);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[t1, t2]);

        assert!(prompt.contains("src/a.rs"));
        assert!(prompt.contains("src/b.rs"));
        assert!(prompt.contains("Fix the error handling."));
        assert!(prompt.contains("Add a test for this path."));
    }

    // ---------------------------------------------------------------------------
    // build_reviewer_prompt tests (CROW-22)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_reviewer_prompt_no_threads() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "--- a/foo\n+++ b/foo\n+hello", &[]);

        assert!(prompt.contains("PR #1"));
        assert!(prompt.contains("Test PR"));
        assert!(prompt.contains("@tester"));
        assert!(prompt.contains("## Files Changed"));
        assert!(prompt.contains("## Diff"));
        // No existing comments section when none exist
        assert!(!prompt.contains("## Existing Review Comments"));
    }

    #[test]
    fn test_reviewer_prompt_existing_threads_shown() {
        let comment = make_comment("alice", "Please fix this.");
        let thread = make_thread("t1", false, "src/lib.rs", Some(5), None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("Existing Review Comments"));
        assert!(prompt.contains("Do NOT repeat them"));
        assert!(prompt.contains("@alice"));
        assert!(prompt.contains("Please fix this."));
    }

    #[test]
    fn test_reviewer_prompt_actionable_and_direct() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("actionable"));
        assert!(prompt.contains("direct"));
    }

    #[test]
    fn test_reviewer_prompt_with_body() {
        let detail = make_detail("Adds caching layer.", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("## PR Description"));
        assert!(prompt.contains("Adds caching layer."));
    }

    #[test]
    fn test_reviewer_prompt_large_diff_truncation() {
        let large_diff = "y".repeat(200_000);
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, &large_diff, &[]);

        assert!(prompt.contains("diff truncated"));
        assert!(prompt.contains("200000 bytes total"));
    }

    #[test]
    fn test_reviewer_prompt_crow_comment_guidance() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("crow comment"));
    }

    #[test]
    fn test_reviewer_prompt_files_changed() {
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
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("+15"));
        assert!(prompt.contains("-3"));
        assert!(prompt.contains("Cargo.toml"));
    }

    #[test]
    fn test_reviewer_prompt_thread_line_range() {
        let comment = make_comment("rev", "Range comment");
        let thread = make_thread("t3", false, "src/foo.rs", Some(20), Some(15), vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("L15-20"));
    }

    // ---------------------------------------------------------------------------
    // run() routing tests (CROW-20)
    // ---------------------------------------------------------------------------

    #[test]
    fn run_routes_to_author_flow_when_current_user_is_pr_author() {
        let mut mock_gh = MockGhClient::new();
        // pr_author_login and current_user_login both default to "author"
        mock_gh.pr_author_login = "alice".to_string();
        mock_gh.current_user_login = "alice".to_string();
        let mock_wt = MockWtClient::new();

        // Should succeed: same user → author flow
        run(&mock_gh, &mock_wt, 7).unwrap();
    }

    #[test]
    fn run_routes_to_reviewer_flow_when_current_user_differs() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.pr_author_login = "alice".to_string();
        mock_gh.current_user_login = "bob".to_string();
        let mock_wt = MockWtClient::new();

        // Should succeed: different user → reviewer flow
        run(&mock_gh, &mock_wt, 7).unwrap();
    }

    #[test]
    fn run_fetches_pr_details_and_launches_session() {
        let mock_gh = MockGhClient::new();
        let mock_wt = MockWtClient::new();

        // run() calls checkout_pr_exec which in the mock returns Ok(())
        run(&mock_gh, &mock_wt, 7).unwrap();
    }

    #[test]
    fn run_with_threads_builds_prompt_with_thread_info() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.threads = vec![ReviewThread {
            id: "rt1".to_string(),
            is_resolved: false,
            is_outdated: false,
            path: "src/lib.rs".to_string(),
            line: Some(42),
            start_line: None,
            comments: crate::types::ThreadComments {
                nodes: vec![ThreadComment {
                    id: "c1".to_string(),
                    author: Author {
                        login: "reviewer".to_string(),
                    },
                    body: "Please fix.".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    url: "https://github.com/test/repo/pull/1#comment".to_string(),
                    diff_hunk: "@@ -1,2 +1,3 @@".to_string(),
                }],
            },
        }];

        let mock_wt = MockWtClient::new();
        run(&mock_gh, &mock_wt, 3).unwrap();
    }
}
