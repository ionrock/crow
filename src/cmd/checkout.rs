use anyhow::{Context, Result};

use crate::cmd::reviews;
use crate::gh::GhClient;
use crate::wt::WtClient;

pub fn run(gh: &dyn GhClient, wt: &dyn WtClient, pr: u64) -> Result<()> {
    wt.checkout_pr(pr)
        .context("Failed to check out PR worktree")?;

    println!("Checked out PR #{} into worktree.\n", pr);

    // Auto-show unresolved review threads
    reviews::run(gh, Some(pr), false, false, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{MockGhClient, MockWtClient};

    #[test]
    fn successful_checkout_records_pr_and_shows_reviews() {
        let mut mock_gh = MockGhClient::new();
        mock_gh.threads = vec![];

        let mock_wt = MockWtClient::new();

        run(&mock_gh, &mock_wt, 42).unwrap();

        assert_eq!(mock_wt.checked_out_pr.get(), Some(42));
    }
}
