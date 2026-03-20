use std::collections::BTreeMap;

use anyhow::Result;

use crate::display;
use crate::gh::GhClient;

pub fn run(
    gh: &dyn GhClient,
    pr: Option<u64>,
    all: bool,
    diff: bool,
    _unresolved: bool,
) -> Result<()> {
    let pr_number = match pr {
        Some(n) => n,
        None => gh.current_pr_number()?,
    };

    let repo = gh.repo_info()?;
    let threads = gh.review_threads(repo.owner_login(), &repo.name, pr_number)?;

    // Filter threads
    let threads: Vec<_> = if all {
        threads
    } else {
        threads.into_iter().filter(|t| !t.is_resolved).collect()
    };

    if threads.is_empty() {
        if all {
            println!("No review threads on PR #{}.", pr_number);
        } else {
            println!("No unresolved review threads on PR #{}.", pr_number);
        }
        return Ok(());
    }

    // Group by file path
    let mut by_file: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for thread in threads {
        by_file.entry(thread.path.clone()).or_default().push(thread);
    }

    // Sort threads within each file by line number
    for threads in by_file.values_mut() {
        threads.sort_by_key(|t| t.line.unwrap_or(0));
    }

    for (path, threads) in &by_file {
        let unresolved_count = threads.iter().filter(|t| !t.is_resolved).count();
        display::section_header(&format!("{} ({} unresolved)", path, unresolved_count));

        for thread in threads {
            display::print_thread(thread, diff);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockGhClient;
    use crate::types::{Author, ReviewThread, ThreadComment, ThreadComments};

    fn make_thread(id: &str, resolved: bool, path: &str) -> ReviewThread {
        ReviewThread {
            id: id.to_string(),
            is_resolved: resolved,
            is_outdated: false,
            path: path.to_string(),
            line: Some(10),
            start_line: None,
            comments: ThreadComments {
                nodes: vec![ThreadComment {
                    id: "comment-1".to_string(),
                    author: Author {
                        login: "reviewer".to_string(),
                    },
                    body: "Please fix this.".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    url: "https://github.com/org/repo/pull/1#comment-1".to_string(),
                    diff_hunk: "@@ -1,3 +1,4 @@\n+new line".to_string(),
                }],
            },
        }
    }

    #[test]
    fn no_threads_with_explicit_pr_number() {
        let mut mock = MockGhClient::new();
        mock.threads = vec![];
        run(&mock, Some(42), false, false, true).unwrap();
    }

    #[test]
    fn falls_back_to_current_pr_number() {
        let mut mock = MockGhClient::new();
        mock.current_pr = 99;
        mock.threads = vec![];
        run(&mock, None, false, false, true).unwrap();
    }

    #[test]
    fn unresolved_threads_shown_without_all_flag() {
        let mut mock = MockGhClient::new();
        mock.threads = vec![
            make_thread("t1", false, "src/main.rs"),
            make_thread("t2", true, "src/lib.rs"),
        ];
        // Without --all, only unresolved threads shown
        run(&mock, Some(1), false, false, true).unwrap();
    }

    #[test]
    fn all_flag_shows_resolved_threads() {
        let mut mock = MockGhClient::new();
        mock.threads = vec![
            make_thread("t1", false, "src/main.rs"),
            make_thread("t2", true, "src/lib.rs"),
        ];
        // With --all, resolved threads are also shown
        run(&mock, Some(1), true, false, true).unwrap();
    }

    #[test]
    fn diff_flag_shows_diff_hunks() {
        let mut mock = MockGhClient::new();
        mock.threads = vec![make_thread("t1", false, "src/main.rs")];
        run(&mock, Some(1), false, true, true).unwrap();
    }

    #[test]
    fn no_threads_all_flag_prints_appropriate_message() {
        let mut mock = MockGhClient::new();
        mock.threads = vec![];
        // With --all and no threads, different message
        run(&mock, Some(5), true, false, true).unwrap();
    }

    #[test]
    fn no_unresolved_threads_prints_appropriate_message() {
        let mut mock = MockGhClient::new();
        // Only resolved threads — filtered out without --all
        mock.threads = vec![make_thread("t1", true, "src/main.rs")];
        run(&mock, Some(5), false, false, true).unwrap();
    }
}
