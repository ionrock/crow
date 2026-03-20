use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::io::Read;

use crate::cli::ReviewEvent;
use crate::gh::GhClient;

pub fn run(gh: &dyn GhClient, pr: u64, event: ReviewEvent, body: Option<String>) -> Result<()> {
    let body = match body {
        Some(b) => b,
        None => {
            // Open editor for body
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            let tmp = std::env::temp_dir().join("crow_review_body.md");
            std::fs::write(&tmp, "").context("Failed to create temp file")?;

            let status = std::process::Command::new(&editor)
                .arg(&tmp)
                .status()
                .with_context(|| format!("Failed to open editor: {}", editor))?;

            if !status.success() {
                anyhow::bail!("Editor exited with non-zero status");
            }

            let mut content = String::new();
            std::fs::File::open(&tmp)
                .context("Failed to read editor output")?
                .read_to_string(&mut content)?;

            let _ = std::fs::remove_file(&tmp);

            let content = content.trim().to_string();
            if content.is_empty() {
                anyhow::bail!("Empty review body — aborting");
            }
            content
        }
    };

    let event_flag = match event {
        ReviewEvent::Approve => "approve",
        ReviewEvent::RequestChanges => "request-changes",
        ReviewEvent::Comment => "comment",
    };

    gh.post_review(pr, event_flag, &body)?;

    let action = match event {
        ReviewEvent::Approve => "Approved",
        ReviewEvent::RequestChanges => "Requested changes on",
        ReviewEvent::Comment => "Commented on",
    };

    println!("{} PR #{}.", action.green(), pr);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockGhClient;

    #[test]
    fn approve_event_posts_approve_review() {
        let mock = MockGhClient::new();
        run(&mock, 5, ReviewEvent::Approve, Some("LGTM".to_string())).unwrap();
        let reviews = mock.reviews.borrow();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].event, "approve");
        assert_eq!(reviews[0].body, "LGTM");
    }

    #[test]
    fn request_changes_event_maps_correctly() {
        let mock = MockGhClient::new();
        run(
            &mock,
            3,
            ReviewEvent::RequestChanges,
            Some("Please fix X.".to_string()),
        )
        .unwrap();
        let reviews = mock.reviews.borrow();
        assert_eq!(reviews[0].event, "request-changes");
    }

    #[test]
    fn comment_event_maps_correctly() {
        let mock = MockGhClient::new();
        run(&mock, 7, ReviewEvent::Comment, Some("A note.".to_string())).unwrap();
        let reviews = mock.reviews.borrow();
        assert_eq!(reviews[0].event, "comment");
    }

    #[test]
    fn body_provided_skips_editor() {
        let mock = MockGhClient::new();
        run(
            &mock,
            1,
            ReviewEvent::Comment,
            Some("body text".to_string()),
        )
        .unwrap();
        let reviews = mock.reviews.borrow();
        assert_eq!(reviews[0].body, "body text");
    }
}
