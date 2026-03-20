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
