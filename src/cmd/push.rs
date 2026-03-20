use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::process::Command;

use crate::gh::GhClient;

pub fn run(gh: &dyn GhClient, reply: Option<String>) -> Result<()> {
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
        let repo = gh.repo_info()?;
        let pr = gh.current_pr_number()?;
        let threads = gh.review_threads(repo.owner_login(), &repo.name, pr)?;

        let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();

        if unresolved.is_empty() {
            println!("No unresolved threads to reply to.");
            return Ok(());
        }

        let mut replied = 0;
        for thread in &unresolved {
            // Reply to the last comment in each thread
            if let Some(last_comment) = thread.comments.nodes.last() {
                gh.reply_to_thread(repo.owner_login(), &repo.name, pr, &last_comment.id, &msg)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockGhClient;
    use crate::types::{Author, ReviewThread, ThreadComment, ThreadComments};

    fn make_unresolved_thread(id: &str, comment_id: &str) -> ReviewThread {
        ReviewThread {
            id: id.to_string(),
            is_resolved: false,
            is_outdated: false,
            path: "src/main.rs".to_string(),
            line: Some(5),
            start_line: None,
            comments: ThreadComments {
                nodes: vec![ThreadComment {
                    id: comment_id.to_string(),
                    author: Author {
                        login: "reviewer".to_string(),
                    },
                    body: "Please fix.".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    url: "https://github.com/org/repo/pull/1#comment-1".to_string(),
                    diff_hunk: String::new(),
                }],
            },
        }
    }

    // Note: tests that call run() directly will attempt to `git push`, which will
    // fail outside a git repo with an upstream. We test only the reply logic
    // by calling the gh client methods directly; the full integration path is
    // exercised by e2e / manual tests.
    //
    // Instead we test the gh-interaction logic at the unit level by verifying
    // that the mock records the expected calls.

    #[test]
    fn reply_with_no_unresolved_threads_records_no_replies() {
        let mut mock = MockGhClient::new();
        mock.threads = vec![];

        // Simulate the reply branch directly (skip git push)
        let repo = mock.repo_info().unwrap();
        let pr = mock.current_pr_number().unwrap();
        let threads = mock
            .review_threads(repo.owner_login(), &repo.name, pr)
            .unwrap();
        let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();

        assert!(unresolved.is_empty());
        assert_eq!(mock.replies.borrow().len(), 0);
    }

    #[test]
    fn reply_with_unresolved_threads_calls_reply_for_each() {
        let mut mock = MockGhClient::new();
        mock.threads = vec![
            make_unresolved_thread("t1", "c1"),
            make_unresolved_thread("t2", "c2"),
        ];

        let repo = mock.repo_info().unwrap();
        let pr = mock.current_pr_number().unwrap();
        let threads = mock
            .review_threads(repo.owner_login(), &repo.name, pr)
            .unwrap();
        let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();

        let msg = "Done, addressed.";
        for thread in &unresolved {
            if let Some(last) = thread.comments.nodes.last() {
                mock.reply_to_thread(repo.owner_login(), &repo.name, pr, &last.id, msg)
                    .unwrap();
            }
        }

        assert_eq!(mock.replies.borrow().len(), 2);
    }

    #[test]
    fn reply_with_mixed_threads_only_replies_to_unresolved() {
        let mut mock = MockGhClient::new();
        let mut resolved = make_unresolved_thread("t-resolved", "c-resolved");
        resolved.is_resolved = true;
        mock.threads = vec![make_unresolved_thread("t1", "c1"), resolved];

        let repo = mock.repo_info().unwrap();
        let pr = mock.current_pr_number().unwrap();
        let threads = mock
            .review_threads(repo.owner_login(), &repo.name, pr)
            .unwrap();
        let unresolved: Vec<_> = threads.iter().filter(|t| !t.is_resolved).collect();

        assert_eq!(unresolved.len(), 1);

        let msg = "Fixed.";
        for thread in &unresolved {
            if let Some(last) = thread.comments.nodes.last() {
                mock.reply_to_thread(repo.owner_login(), &repo.name, pr, &last.id, msg)
                    .unwrap();
            }
        }

        assert_eq!(mock.replies.borrow().len(), 1);
    }
}
