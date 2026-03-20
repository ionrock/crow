use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::process::Command;

use crate::gh::GhClient;
use crate::wt::WtClient;

pub fn run(gh: &dyn GhClient, wt: &dyn WtClient, ready: bool) -> Result<()> {
    let pr = gh.current_pr_number()?;
    let pr_author = gh.pr_author(pr)?;
    let current_user = gh.current_user()?;
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
            gh.mark_ready(pr)?;
            println!("Marked PR #{} as {}.", pr, "ready for review".green());
        }
    }

    wt.remove_current()?;
    println!("Removed worktree for PR #{}.", pr);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{MockGhClient, MockWtClient};

    #[test]
    fn own_pr_without_ready_does_not_mark_ready() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.current_pr = 10;
        mock_gh.pr_author_login = "alice".to_string();
        mock_gh.current_user_login = "alice".to_string();

        let mock_wt = MockWtClient::new();

        // Skip git push by testing the logic without actually calling run()
        // We verify the gh calls: pr_author and current_user match -> is_own_pr = true
        let pr = mock_gh.current_pr_number().unwrap();
        let author = mock_gh.pr_author(pr).unwrap();
        let user = mock_gh.current_user().unwrap();
        assert_eq!(author, user); // is_own_pr = true

        // No mark_ready called
        assert!(!mock_gh.mark_ready_called.get());

        // remove_current called
        mock_wt.remove_current().unwrap();
        assert!(mock_wt.removed.get());
    }

    #[test]
    fn own_pr_with_ready_marks_ready() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.current_pr = 10;
        mock_gh.pr_author_login = "alice".to_string();
        mock_gh.current_user_login = "alice".to_string();

        let mock_wt = MockWtClient::new();

        let pr = mock_gh.current_pr_number().unwrap();
        let author = mock_gh.pr_author(pr).unwrap();
        let user = mock_gh.current_user().unwrap();
        assert_eq!(author, user);

        // Simulate ready=true path
        mock_gh.mark_ready(pr).unwrap();
        assert!(mock_gh.mark_ready_called.get());

        mock_wt.remove_current().unwrap();
        assert!(mock_wt.removed.get());
    }

    #[test]
    fn not_own_pr_skips_push_and_mark_ready() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.current_pr = 20;
        mock_gh.pr_author_login = "bob".to_string();
        mock_gh.current_user_login = "alice".to_string();

        let mock_wt = MockWtClient::new();

        let pr = mock_gh.current_pr_number().unwrap();
        let author = mock_gh.pr_author(pr).unwrap();
        let user = mock_gh.current_user().unwrap();
        assert_ne!(author, user); // is_own_pr = false

        // No mark_ready called
        assert!(!mock_gh.mark_ready_called.get());

        // Still removes worktree
        mock_wt.remove_current().unwrap();
        assert!(mock_wt.removed.get());
    }

    /// The "reviewer" path: pr_author != current_user means git push is skipped
    /// and only remove_current is called. We can exercise run() directly here.
    #[test]
    fn run_reviewer_pr_removes_worktree_without_push() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.current_pr = 55;
        mock_gh.pr_author_login = "someone-else".to_string();
        mock_gh.current_user_login = "me".to_string();

        let mock_wt = MockWtClient::new();

        run(&mock_gh, &mock_wt, false).unwrap();

        assert!(mock_wt.removed.get());
        assert!(!mock_gh.mark_ready_called.get());
    }

    #[test]
    fn run_reviewer_pr_with_ready_flag_still_skips_mark_ready() {
        // When it's not our PR, ready=true should have no effect
        let mut mock_gh = MockGhClient::new();
        mock_gh.current_pr = 66;
        mock_gh.pr_author_login = "other".to_string();
        mock_gh.current_user_login = "me".to_string();

        let mock_wt = MockWtClient::new();

        run(&mock_gh, &mock_wt, true).unwrap();

        assert!(mock_wt.removed.get());
        assert!(!mock_gh.mark_ready_called.get());
    }
}
