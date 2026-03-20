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

    /// When body is None, we open an editor. Use a write-then-true script so the
    /// temp file has content when we read it back. We rely on a small shell script
    /// as EDITOR that writes a fixed string to the file it receives as $1.
    #[test]
    fn no_body_opens_editor_and_reads_content() {
        // Build a tiny shell script that writes a known string into $1
        let script_dir = tempfile::tempdir().expect("tmpdir");
        let script = script_dir.path().join("fake_editor.sh");
        std::fs::write(&script, "#!/bin/sh\necho 'review content' > \"$1\"\n").unwrap();
        // Make it executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        let mock = MockGhClient::new();
        // Point EDITOR at our fake script
        std::env::set_var("EDITOR", script.to_str().unwrap());

        run(&mock, 2, ReviewEvent::Approve, None).unwrap();

        let reviews = mock.reviews.borrow();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].event, "approve");
        assert!(reviews[0].body.contains("review content"));

        // Restore
        std::env::remove_var("EDITOR");
    }

    /// When the editor produces an empty file, run() returns an error.
    #[test]
    fn no_body_empty_editor_output_returns_error() {
        // Use /usr/bin/true as editor — it succeeds but writes nothing
        let mock = MockGhClient::new();
        std::env::set_var("EDITOR", "/usr/bin/true");

        let result = run(&mock, 3, ReviewEvent::Comment, None);
        assert!(result.is_err());
        let msg = format!("{:#}", result.unwrap_err());
        assert!(msg.contains("Empty review body"), "got: {}", msg);

        std::env::remove_var("EDITOR");
    }
}
