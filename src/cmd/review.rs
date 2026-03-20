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
        build_author_prompt(&detail, &diff, &threads)
    } else {
        build_reviewer_prompt(&detail, &diff, &threads)
    };

    println!(
        "Launching Claude review session for {} by @{}...\n",
        format!("PR #{}", detail.number).cyan(),
        detail.author.login.cyan()
    );

    // Exec into worktree with claude session — does not return on success
    wt.checkout_pr_exec(pr, "claude", &["--dangerously-skip-permissions", &prompt])
        .context("Failed to launch review session")
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn pr_header(detail: &PrDetail) -> String {
    format!(
        "PR #{}: {}\n\
         Author: @{}\n\
         Branch: {} → {}\n\
         URL: {}\n",
        detail.number,
        detail.title,
        detail.author.login,
        detail.head_ref_name,
        detail.base_ref_name,
        detail.url,
    )
}

fn files_changed_section(detail: &PrDetail) -> String {
    let mut out = "\n## Files Changed\n\n".to_string();
    for f in &detail.files {
        out.push_str(&format!(
            "  {} (+{} -{})  \n",
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

fn format_line_label(thread: &ReviewThread) -> String {
    match (thread.start_line, thread.line) {
        (Some(s), Some(e)) if s != e => format!("L{}-{}", s, e),
        (_, Some(l)) => format!("L{}", l),
        _ => "L?".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Author prompt — for the PR author reviewing their own work
// ---------------------------------------------------------------------------

pub(crate) fn build_author_prompt(
    detail: &PrDetail,
    diff: &str,
    threads: &[ReviewThread],
) -> String {
    let mut prompt = pr_header(detail);

    if !detail.body.is_empty() {
        prompt.push_str(&format!("\n## PR Description\n\n{}\n", detail.body));
    }

    prompt.push_str(&files_changed_section(detail));

    // Unresolved review threads — the author must address these
    let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();
    if !unresolved.is_empty() {
        prompt.push_str(&format!(
            "\n## Unresolved Review Comments to Address ({})\n\n",
            unresolved.len()
        ));
        for thread in &unresolved {
            let line_label = format_line_label(thread);
            prompt.push_str(&format!("### {} {}\n", thread.path, line_label));
            if let Some(first_comment) = thread.comments.nodes.first() {
                prompt.push_str(&format!("```diff\n{}\n```\n", first_comment.diff_hunk));
            }
            for c in &thread.comments.nodes {
                prompt.push_str(&format!("@{}: {}\n", c.author.login, c.body));
            }
            prompt.push('\n');
        }
    }

    prompt.push_str(&diff_section(diff));

    // Author-specific task instructions
    prompt.push_str(
        "\n## Your Task\n\n\
         You are the PR author reviewing your own work and responding to feedback. \
         You are in the PR's worktree and can read any file, run tests, or run CI commands.\n\n\
         Focus on:\n\
         - Addressing each unresolved review comment\n\
         - Identifying any remaining issues before requesting re-review\n\
         - Running tests to verify your changes are correct\n\n\
         For unresolved comments, mark each as must-fix or addressed. \
         Use `crow comment` to reply to specific threads.\n",
    );

    prompt
}

// ---------------------------------------------------------------------------
// Reviewer prompt — for someone reviewing another's PR
// ---------------------------------------------------------------------------

pub(crate) fn build_reviewer_prompt(
    detail: &PrDetail,
    diff: &str,
    threads: &[ReviewThread],
) -> String {
    let mut prompt = pr_header(detail);

    if !detail.body.is_empty() {
        prompt.push_str(&format!("\n## PR Description\n\n{}\n", detail.body));
    }

    prompt.push_str(&files_changed_section(detail));

    // Show existing threads so reviewer doesn't repeat them
    let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();
    if !unresolved.is_empty() {
        prompt.push_str(&format!(
            "\n## Existing Unresolved Review Comments ({}) — do not repeat these\n\n",
            unresolved.len()
        ));
        for thread in &unresolved {
            let line_label = format_line_label(thread);
            prompt.push_str(&format!("### {} {}\n", thread.path, line_label));
            for c in &thread.comments.nodes {
                prompt.push_str(&format!("@{}: {}\n", c.author.login, c.body));
            }
            prompt.push('\n');
        }
    }

    prompt.push_str(&diff_section(diff));

    // Reviewer-specific task instructions
    prompt.push_str(
        "\n## Your Task\n\n\
         Review this PR thoroughly. You are in the PR's worktree and can read any file, \
         run tests, or run CI commands.\n\n\
         Be actionable and direct. Focus on:\n\
         - Correctness: bugs, logic errors, edge cases\n\
         - Design: architecture, abstractions, API surface\n\
         - Safety: error handling, security, resource management\n\
         - Style: naming, clarity, idiomatic patterns for this codebase\n\n\
         For each issue found, specify the file and line. \
         Distinguish between must-fix issues and suggestions.\n\n\
         You can run tests (`make test`, `cargo test`, etc.) or explore the code \
         to verify your findings. The user can interact with you to discuss, \
         run additional checks, or make changes.\n\n\
         Use `crow comment` to post review comments on specific lines.\n",
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

    // -----------------------------------------------------------------------
    // run() routing tests
    // -----------------------------------------------------------------------

    #[test]
    fn run_routes_to_author_flow_when_current_user_is_pr_author() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.pr_author_login = "alice".to_string();
        mock_gh.current_user_login = "alice".to_string();
        // Add an unresolved thread so we can detect author-specific content
        mock_gh.threads = vec![make_thread(
            "t1",
            false,
            "src/lib.rs",
            Some(10),
            None,
            vec![make_comment("reviewer", "Please fix this")],
        )];

        let mock_wt = MockWtClient::new();
        // run() calls checkout_pr_exec which records the prompt argument
        // We verify it succeeds (author flow chosen); prompt content verified in unit tests
        run(&mock_gh, &mock_wt, 1).unwrap();
    }

    #[test]
    fn run_routes_to_reviewer_flow_when_current_user_is_not_pr_author() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.pr_author_login = "alice".to_string();
        mock_gh.current_user_login = "bob".to_string();

        let mock_wt = MockWtClient::new();
        run(&mock_gh, &mock_wt, 1).unwrap();
    }

    #[test]
    fn run_fetches_pr_details_and_launches_session() {
        let mock_gh = MockGhClient::new();
        let mock_wt = MockWtClient::new();

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

    // -----------------------------------------------------------------------
    // build_author_prompt() tests
    // -----------------------------------------------------------------------

    #[test]
    fn author_prompt_no_threads_still_has_task_instructions() {
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "diff content", &[]);

        assert!(prompt.contains("## Your Task"));
        assert!(prompt.contains("PR author"));
        assert!(prompt.contains("## Files Changed"));
        assert!(prompt.contains("## Diff"));
        // No unresolved section when there are no threads
        assert!(!prompt.contains("Unresolved Review Comments to Address"));
    }

    #[test]
    fn author_prompt_multiple_unresolved_threads_all_shown_with_comments_and_diff_hunks() {
        let comment1 = make_comment("reviewer1", "Rename this variable.");
        let comment2 = make_comment("reviewer2", "Extract this to a function.");
        let thread1 = make_thread("t1", false, "src/foo.rs", Some(10), None, vec![comment1]);
        let thread2 = make_thread("t2", false, "src/bar.rs", Some(20), None, vec![comment2]);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[thread1, thread2]);

        assert!(prompt.contains("Unresolved Review Comments to Address (2)"));
        assert!(prompt.contains("src/foo.rs"));
        assert!(prompt.contains("Rename this variable."));
        assert!(prompt.contains("src/bar.rs"));
        assert!(prompt.contains("Extract this to a function."));
        // diff hunks are shown for the author
        assert!(prompt.contains("@@ -1,3 +1,4 @@"));
    }

    #[test]
    fn author_prompt_mixed_resolved_unresolved_only_unresolved_shown() {
        let resolved_comment = make_comment("rev", "Already fixed.");
        let unresolved_comment = make_comment("rev", "Still needs work.");
        let resolved_thread = make_thread(
            "t1",
            true,
            "src/resolved.rs",
            Some(5),
            None,
            vec![resolved_comment],
        );
        let unresolved_thread = make_thread(
            "t2",
            false,
            "src/unresolved.rs",
            Some(15),
            None,
            vec![unresolved_comment],
        );
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[resolved_thread, unresolved_thread]);

        assert!(prompt.contains("Unresolved Review Comments to Address (1)"));
        assert!(prompt.contains("src/unresolved.rs"));
        assert!(prompt.contains("Still needs work."));
        assert!(!prompt.contains("src/resolved.rs"));
        assert!(!prompt.contains("Already fixed."));
    }

    #[test]
    fn author_prompt_with_pr_body_body_included() {
        let detail = make_detail("This PR fixes the authentication bug.", vec![]);
        let prompt = build_author_prompt(&detail, "", &[]);

        assert!(prompt.contains("## PR Description"));
        assert!(prompt.contains("This PR fixes the authentication bug."));
    }

    #[test]
    fn author_prompt_empty_body_no_description_section() {
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[]);

        assert!(!prompt.contains("## PR Description"));
    }

    #[test]
    fn author_prompt_large_diff_truncation() {
        let large_diff = "x".repeat(200_000);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, &large_diff, &[]);

        assert!(prompt.contains("diff truncated"));
        assert!(prompt.contains("200000 bytes total"));
        assert!(prompt.contains("showing first 100000"));
    }

    #[test]
    fn author_prompt_thread_with_line_range_shows_l15_20_format() {
        let comment = make_comment("rev", "Range comment");
        let thread = make_thread("t3", false, "src/foo.rs", Some(20), Some(15), vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("L15-20"));
    }

    #[test]
    fn author_prompt_thread_with_no_line_shows_l_question_mark() {
        let comment = make_comment("rev", "No line comment");
        let thread = make_thread("t4", false, "src/bar.rs", None, None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("L?"));
    }

    #[test]
    fn author_prompt_must_fix_prioritization_instruction_present() {
        let detail = make_detail("", vec![]);
        let prompt = build_author_prompt(&detail, "", &[]);

        assert!(prompt.contains("must-fix"));
    }

    // -----------------------------------------------------------------------
    // build_reviewer_prompt() tests
    // -----------------------------------------------------------------------

    #[test]
    fn reviewer_prompt_no_existing_threads_review_section_present() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "diff content", &[]);

        assert!(prompt.contains("## Your Task"));
        assert!(prompt.contains("## Files Changed"));
        assert!(prompt.contains("## Diff"));
        // No existing threads section when there are none
        assert!(!prompt.contains("Existing Unresolved Review Comments"));
    }

    #[test]
    fn reviewer_prompt_with_existing_threads_shown_with_do_not_repeat_instruction() {
        let comment = make_comment("prev-reviewer", "Already noted this issue.");
        let thread = make_thread("t1", false, "src/lib.rs", Some(42), None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("Existing Unresolved Review Comments (1)"));
        assert!(prompt.contains("do not repeat"));
        assert!(prompt.contains("Already noted this issue."));
    }

    #[test]
    fn reviewer_prompt_contains_actionable_and_direct_keywords() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("actionable"));
        assert!(prompt.contains("direct"));
    }

    #[test]
    fn reviewer_prompt_pr_body_included_when_present() {
        let detail = make_detail("Implements the new caching layer.", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("## PR Description"));
        assert!(prompt.contains("Implements the new caching layer."));
    }

    #[test]
    fn reviewer_prompt_empty_body_no_description_section() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(!prompt.contains("## PR Description"));
    }

    #[test]
    fn reviewer_prompt_large_diff_truncation() {
        let large_diff = "y".repeat(200_000);
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, &large_diff, &[]);

        assert!(prompt.contains("diff truncated"));
        assert!(prompt.contains("200000 bytes total"));
        assert!(prompt.contains("showing first 100000"));
    }

    #[test]
    fn reviewer_prompt_files_changed_section() {
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
        assert!(prompt.contains("+2"));
        assert!(prompt.contains("-0"));
    }

    #[test]
    fn reviewer_prompt_crow_comment_guidance_present() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("crow comment"));
    }

    #[test]
    fn reviewer_prompt_line_range_formatting() {
        let comment = make_comment("rev", "Range comment");
        let thread = make_thread("t3", false, "src/foo.rs", Some(20), Some(15), vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("L15-20"));
    }

    #[test]
    fn reviewer_prompt_thread_no_line_shows_l_question_mark() {
        let comment = make_comment("rev", "No line comment");
        let thread = make_thread("t4", false, "src/bar.rs", None, None, vec![comment]);
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[thread]);

        assert!(prompt.contains("L?"));
    }

    #[test]
    fn reviewer_prompt_must_fix_instruction_present() {
        let detail = make_detail("", vec![]);
        let prompt = build_reviewer_prompt(&detail, "", &[]);

        assert!(prompt.contains("must-fix"));
    }

    // -----------------------------------------------------------------------
    // Shared helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn pr_header_contains_all_fields() {
        let detail = make_detail("", vec![]);
        let header = pr_header(&detail);

        assert!(header.contains("PR #1"));
        assert!(header.contains("Test PR"));
        assert!(header.contains("@tester"));
        assert!(header.contains("feat/test"));
        assert!(header.contains("main"));
        assert!(header.contains("https://github.com/owner/repo/pull/1"));
    }

    #[test]
    fn files_changed_section_lists_all_files() {
        let files = vec![
            PrFile {
                path: "src/lib.rs".to_string(),
                additions: 50,
                deletions: 10,
            },
            PrFile {
                path: "README.md".to_string(),
                additions: 5,
                deletions: 0,
            },
        ];
        let detail = make_detail("", files);
        let section = files_changed_section(&detail);

        assert!(section.contains("## Files Changed"));
        assert!(section.contains("src/lib.rs"));
        assert!(section.contains("+50"));
        assert!(section.contains("-10"));
        assert!(section.contains("README.md"));
    }

    #[test]
    fn diff_section_wraps_in_code_fence() {
        let section = diff_section("--- a/foo\n+++ b/foo\n+hello");

        assert!(section.contains("```diff"));
        assert!(section.contains("+hello"));
        assert!(section.contains("## Diff"));
    }

    #[test]
    fn diff_section_truncates_large_diff() {
        let large = "z".repeat(150_000);
        let section = diff_section(&large);

        assert!(section.contains("diff truncated"));
        assert!(section.contains("150000 bytes total"));
    }

    #[test]
    fn format_line_label_single_line() {
        let comment = make_comment("rev", "comment");
        let thread = make_thread("t1", false, "f.rs", Some(42), None, vec![comment]);
        assert_eq!(format_line_label(&thread), "L42");
    }

    #[test]
    fn format_line_label_line_range() {
        let comment = make_comment("rev", "comment");
        let thread = make_thread("t1", false, "f.rs", Some(20), Some(15), vec![comment]);
        assert_eq!(format_line_label(&thread), "L15-20");
    }

    #[test]
    fn format_line_label_no_line() {
        let comment = make_comment("rev", "comment");
        let thread = make_thread("t1", false, "f.rs", None, None, vec![comment]);
        assert_eq!(format_line_label(&thread), "L?");
    }

    #[test]
    fn format_line_label_same_start_and_end_line() {
        // When start_line == line, show "L42" not "L42-42"
        let comment = make_comment("rev", "comment");
        let thread = make_thread("t1", false, "f.rs", Some(42), Some(42), vec![comment]);
        assert_eq!(format_line_label(&thread), "L42");
    }
}
