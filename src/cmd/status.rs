use anyhow::Result;

use crate::display;
use crate::gh::GhClient;

pub fn run(gh: &dyn GhClient) -> Result<()> {
    let authored = gh.pr_list_authored()?;
    let review_requested = gh.pr_list_review_requested()?;

    if authored.is_empty() && review_requested.is_empty() {
        println!("No PRs needing attention.");
        return Ok(());
    }

    if !authored.is_empty() {
        display::section_header("Authored PRs:");
        for pr in &authored {
            display::print_pr_row(pr, false);
        }
    }

    if !review_requested.is_empty() {
        display::section_header("Review Requested:");
        for pr in &review_requested {
            display::print_pr_row(pr, true);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockGhClient;
    use crate::types::{Author, Pr};

    fn make_pr(number: u64, title: &str) -> Pr {
        Pr {
            number,
            title: title.to_string(),
            head_ref_name: "feat/branch".to_string(),
            review_decision: None,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            url: format!("https://github.com/org/repo/pull/{}", number),
            author: Some(Author {
                login: "user".to_string(),
            }),
        }
    }

    #[test]
    fn no_prs_prints_no_attention_message() {
        let mut mock = MockGhClient::new();
        mock.authored = vec![];
        mock.review_requested = vec![];
        // Should succeed and not panic
        run(&mock).unwrap();
    }

    #[test]
    fn only_authored_prs_succeeds() {
        let mut mock = MockGhClient::new();
        mock.authored = vec![make_pr(1, "Add feature")];
        mock.review_requested = vec![];
        run(&mock).unwrap();
    }

    #[test]
    fn only_review_requested_prs_succeeds() {
        let mut mock = MockGhClient::new();
        mock.authored = vec![];
        mock.review_requested = vec![make_pr(2, "Fix bug")];
        run(&mock).unwrap();
    }

    #[test]
    fn both_sections_present_succeeds() {
        let mut mock = MockGhClient::new();
        mock.authored = vec![make_pr(1, "Add feature")];
        mock.review_requested = vec![make_pr(2, "Fix bug")];
        run(&mock).unwrap();
    }
}
