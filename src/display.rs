// display.rs — terminal formatting: colors, tables, threads

use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;

use crate::types::{CheckRun, Pr, ReviewThread};

// ---------------------------------------------------------------------------
// Time formatting
// ---------------------------------------------------------------------------

pub fn time_ago(dt: &str) -> String {
    let Ok(parsed) = DateTime::parse_from_rfc3339(dt) else {
        return dt.to_string();
    };

    let now = Utc::now();
    let delta = now.signed_duration_since(parsed.with_timezone(&Utc));

    let secs = delta.num_seconds();
    if secs < 60 {
        return "just now".to_string();
    }

    let mins = delta.num_minutes();
    if mins < 60 {
        return format!("{}m ago", mins);
    }

    let hours = delta.num_hours();
    if hours < 24 {
        return format!("{}h ago", hours);
    }

    let days = delta.num_days();
    if days < 7 {
        return format!("{}d ago", days);
    }

    let weeks = days / 7;
    format!("{}w ago", weeks)
}

// ---------------------------------------------------------------------------
// Status colors
// ---------------------------------------------------------------------------

pub fn status_color(decision: &str) -> String {
    match decision {
        "APPROVED" => "APPROVED".green().to_string(),
        "CHANGES_REQUESTED" => "CHANGES_REQUESTED".yellow().to_string(),
        _ => decision.dimmed().to_string(),
    }
}

pub fn check_icon(state: &str) -> String {
    match state {
        "SUCCESS" | "NEUTRAL" | "SKIPPED" => "✓".green().to_string(),
        "FAILURE" | "ERROR" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" | "STALE"
        | "STARTUP_FAILURE" => "✗".red().to_string(),
        _ => "○".dimmed().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Section header
// ---------------------------------------------------------------------------

pub fn section_header(title: &str) {
    println!("\n{}", title.bold());
}

// ---------------------------------------------------------------------------
// PR row
// ---------------------------------------------------------------------------

pub fn print_pr_row(pr: &Pr, show_author: bool) {
    let number = format!("#{}", pr.number);
    let title = &pr.title;
    let info = if show_author {
        pr.author
            .as_ref()
            .map(|a| format!("@{}", a.login))
            .unwrap_or_default()
    } else {
        pr.review_decision
            .as_deref()
            .map(status_color)
            .unwrap_or_default()
    };
    let ago = time_ago(&pr.updated_at);

    println!(
        "  {}  {:<40} {:<22} {}",
        number.cyan(),
        title,
        info,
        ago.dimmed()
    );
}

// ---------------------------------------------------------------------------
// CI check row
// ---------------------------------------------------------------------------

pub fn print_check_row(check: &CheckRun) {
    let icon = check_icon(&check.state);
    let ago = check
        .completed_at
        .as_deref()
        .map(time_ago)
        .unwrap_or_default();

    println!("    {} {:<30} {}", icon, check.name, ago.dimmed());

    // Show link for failed checks
    if matches!(
        check.state.as_str(),
        "FAILURE" | "ERROR" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE"
    ) && !check.link.is_empty()
    {
        println!("      → {}", check.link.dimmed());
    }
}

// ---------------------------------------------------------------------------
// Review thread display
// ---------------------------------------------------------------------------

pub fn print_thread(thread: &ReviewThread, show_diff: bool) {
    let line_label = match (thread.start_line, thread.line) {
        (Some(start), Some(end)) if start != end => format!("L{}-{}", start, end),
        (_, Some(line)) => format!("L{}", line),
        _ => "L?".to_string(),
    };

    let comments = &thread.comments.nodes;
    if comments.is_empty() {
        return;
    }

    let first = &comments[0];
    let ago = time_ago(&first.created_at);

    println!(
        "\n  {}  {}  {}",
        line_label.yellow(),
        format!("@{}", first.author.login).cyan(),
        ago.dimmed()
    );

    if show_diff && !first.diff_hunk.is_empty() {
        for line in first.diff_hunk.lines() {
            println!("  {} {}", "│".dimmed(), line.dimmed());
        }
    }

    // First comment body
    for line in first.body.lines() {
        println!("  {} {}", "│".dimmed(), line);
    }

    // Replies
    for reply in comments.iter().skip(1) {
        let reply_ago = time_ago(&reply.created_at);
        println!(
            "  {} {} {}",
            "├──".dimmed(),
            format!("@{}", reply.author.login).cyan(),
            reply_ago.dimmed()
        );
        for line in reply.body.lines() {
            println!("  {}   {}", "│".dimmed(), line);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Author, CheckRun, ThreadComment, ThreadComments, WorkflowInfo};
    use chrono::Duration;

    // Helper: produce an RFC3339 timestamp that is `secs` seconds in the past.
    fn ts_ago(secs: i64) -> String {
        (Utc::now() - Duration::seconds(secs)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    // -------------------------------------------------------------------------
    // time_ago
    // -------------------------------------------------------------------------

    #[test]
    fn time_ago_just_now() {
        assert_eq!(time_ago(&ts_ago(0)), "just now");
    }

    #[test]
    fn time_ago_30_seconds() {
        assert_eq!(time_ago(&ts_ago(30)), "just now");
    }

    #[test]
    fn time_ago_exactly_60_seconds_is_minutes() {
        // 60 s => mins == 1, hours < 1  → "1m ago"
        assert_eq!(time_ago(&ts_ago(60)), "1m ago");
    }

    #[test]
    fn time_ago_45_minutes() {
        assert_eq!(time_ago(&ts_ago(45 * 60)), "45m ago");
    }

    #[test]
    fn time_ago_exactly_60_minutes_is_hours() {
        // 3600 s => hours == 1 → "1h ago"
        assert_eq!(time_ago(&ts_ago(3600)), "1h ago");
    }

    #[test]
    fn time_ago_5_hours() {
        assert_eq!(time_ago(&ts_ago(5 * 3600)), "5h ago");
    }

    #[test]
    fn time_ago_exactly_24_hours_is_days() {
        // 86400 s => days == 1 → "1d ago"
        assert_eq!(time_ago(&ts_ago(86400)), "1d ago");
    }

    #[test]
    fn time_ago_3_days() {
        assert_eq!(time_ago(&ts_ago(3 * 86400)), "3d ago");
    }

    #[test]
    fn time_ago_exactly_7_days_is_weeks() {
        // 7 days == 1 week → "1w ago"
        assert_eq!(time_ago(&ts_ago(7 * 86400)), "1w ago");
    }

    #[test]
    fn time_ago_3_weeks() {
        assert_eq!(time_ago(&ts_ago(21 * 86400)), "3w ago");
    }

    #[test]
    fn time_ago_invalid_returns_input() {
        let bad = "not-a-date";
        assert_eq!(time_ago(bad), bad);
    }

    #[test]
    fn time_ago_empty_string_returns_input() {
        assert_eq!(time_ago(""), "");
    }

    // -------------------------------------------------------------------------
    // status_color
    // -------------------------------------------------------------------------

    #[test]
    fn status_color_approved_is_green() {
        let result = status_color("APPROVED");
        // The ANSI-colored string must contain the raw text
        assert!(result.contains("APPROVED"));
        // Green ANSI escape code starts with \x1b[32m
        assert!(result.contains("\x1b[32m"));
    }

    #[test]
    fn status_color_changes_requested_is_yellow() {
        let result = status_color("CHANGES_REQUESTED");
        assert!(result.contains("CHANGES_REQUESTED"));
        // Yellow ANSI escape code starts with \x1b[33m
        assert!(result.contains("\x1b[33m"));
    }

    #[test]
    fn status_color_unknown_is_dimmed() {
        let result = status_color("PENDING");
        assert!(result.contains("PENDING"));
        // Dimmed/faint ANSI escape code: \x1b[2m
        assert!(result.contains("\x1b[2m"));
    }

    #[test]
    fn status_color_empty_is_dimmed() {
        let result = status_color("");
        // Dimmed ANSI code present even for empty string
        assert!(result.contains("\x1b[2m"));
    }

    // -------------------------------------------------------------------------
    // check_icon
    // -------------------------------------------------------------------------

    #[test]
    fn check_icon_success_is_green_checkmark() {
        let result = check_icon("SUCCESS");
        assert!(result.contains('✓'));
        assert!(result.contains("\x1b[32m"));
    }

    #[test]
    fn check_icon_neutral_is_green_checkmark() {
        let result = check_icon("NEUTRAL");
        assert!(result.contains('✓'));
        assert!(result.contains("\x1b[32m"));
    }

    #[test]
    fn check_icon_skipped_is_green_checkmark() {
        let result = check_icon("SKIPPED");
        assert!(result.contains('✓'));
        assert!(result.contains("\x1b[32m"));
    }

    #[test]
    fn check_icon_failure_is_red_x() {
        let result = check_icon("FAILURE");
        assert!(result.contains('✗'));
        assert!(result.contains("\x1b[31m"));
    }

    #[test]
    fn check_icon_error_is_red_x() {
        let result = check_icon("ERROR");
        assert!(result.contains('✗'));
        assert!(result.contains("\x1b[31m"));
    }

    #[test]
    fn check_icon_cancelled_is_red_x() {
        let result = check_icon("CANCELLED");
        assert!(result.contains('✗'));
        assert!(result.contains("\x1b[31m"));
    }

    #[test]
    fn check_icon_timed_out_is_red_x() {
        let result = check_icon("TIMED_OUT");
        assert!(result.contains('✗'));
        assert!(result.contains("\x1b[31m"));
    }

    #[test]
    fn check_icon_action_required_is_red_x() {
        let result = check_icon("ACTION_REQUIRED");
        assert!(result.contains('✗'));
        assert!(result.contains("\x1b[31m"));
    }

    #[test]
    fn check_icon_stale_is_red_x() {
        let result = check_icon("STALE");
        assert!(result.contains('✗'));
        assert!(result.contains("\x1b[31m"));
    }

    #[test]
    fn check_icon_startup_failure_is_red_x() {
        let result = check_icon("STARTUP_FAILURE");
        assert!(result.contains('✗'));
        assert!(result.contains("\x1b[31m"));
    }

    #[test]
    fn check_icon_pending_is_dimmed_circle() {
        let result = check_icon("PENDING");
        assert!(result.contains('○'));
        assert!(result.contains("\x1b[2m"));
    }

    #[test]
    fn check_icon_in_progress_is_dimmed_circle() {
        let result = check_icon("IN_PROGRESS");
        assert!(result.contains('○'));
        assert!(result.contains("\x1b[2m"));
    }

    #[test]
    fn check_icon_unknown_is_dimmed_circle() {
        let result = check_icon("SOMETHING_ELSE");
        assert!(result.contains('○'));
        assert!(result.contains("\x1b[2m"));
    }

    #[test]
    fn check_icon_empty_is_dimmed_circle() {
        let result = check_icon("");
        assert!(result.contains('○'));
        assert!(result.contains("\x1b[2m"));
    }

    // -------------------------------------------------------------------------
    // print_pr_row — verify no panic with valid inputs
    // -------------------------------------------------------------------------

    fn make_pr(review_decision: Option<&str>, author_login: Option<&str>) -> Pr {
        Pr {
            number: 42,
            title: "Test PR title".to_string(),
            head_ref_name: "feature/test".to_string(),
            review_decision: review_decision.map(String::from),
            updated_at: ts_ago(3600),
            url: "https://github.com/owner/repo/pull/42".to_string(),
            author: author_login.map(|l| Author {
                login: l.to_string(),
            }),
        }
    }

    #[test]
    fn print_pr_row_show_decision_does_not_panic() {
        let pr = make_pr(Some("APPROVED"), None);
        print_pr_row(&pr, false);
    }

    #[test]
    fn print_pr_row_show_author_does_not_panic() {
        let pr = make_pr(None, Some("alice"));
        print_pr_row(&pr, true);
    }

    #[test]
    fn print_pr_row_no_author_no_decision_does_not_panic() {
        let pr = make_pr(None, None);
        print_pr_row(&pr, false);
    }

    // -------------------------------------------------------------------------
    // print_check_row — verify no panic
    // -------------------------------------------------------------------------

    fn make_check(state: &str, completed_at: Option<String>, link: &str) -> CheckRun {
        CheckRun {
            name: "ci / build".to_string(),
            state: state.to_string(),
            bucket: "PASS".to_string(),
            description: None,
            workflow: WorkflowInfo {
                name: "CI".to_string(),
            },
            completed_at,
            link: link.to_string(),
        }
    }

    #[test]
    fn print_check_row_success_does_not_panic() {
        let check = make_check("SUCCESS", Some(ts_ago(600)), "");
        print_check_row(&check);
    }

    #[test]
    fn print_check_row_failure_with_link_does_not_panic() {
        let check = make_check(
            "FAILURE",
            Some(ts_ago(120)),
            "https://github.com/owner/repo/actions/runs/1",
        );
        print_check_row(&check);
    }

    #[test]
    fn print_check_row_no_completed_at_does_not_panic() {
        let check = make_check("PENDING", None, "");
        print_check_row(&check);
    }

    // -------------------------------------------------------------------------
    // print_thread — verify no panic
    // -------------------------------------------------------------------------

    fn make_thread(
        comments: Vec<ThreadComment>,
        line: Option<u64>,
        start_line: Option<u64>,
    ) -> ReviewThread {
        ReviewThread {
            id: "RT_1".to_string(),
            is_resolved: false,
            is_outdated: false,
            path: "src/main.rs".to_string(),
            line,
            start_line,
            comments: ThreadComments { nodes: comments },
        }
    }

    fn make_comment(login: &str, body: &str, diff_hunk: &str) -> ThreadComment {
        ThreadComment {
            id: "TC_1".to_string(),
            author: Author {
                login: login.to_string(),
            },
            body: body.to_string(),
            created_at: ts_ago(3600),
            url: "https://github.com".to_string(),
            diff_hunk: diff_hunk.to_string(),
        }
    }

    #[test]
    fn print_thread_empty_comments_does_not_panic() {
        let thread = make_thread(vec![], Some(10), None);
        print_thread(&thread, false);
    }

    #[test]
    fn print_thread_single_comment_does_not_panic() {
        let thread = make_thread(
            vec![make_comment("alice", "Please fix this.", "")],
            Some(42),
            None,
        );
        print_thread(&thread, false);
    }

    #[test]
    fn print_thread_with_diff_hunk_does_not_panic() {
        let thread = make_thread(
            vec![make_comment(
                "bob",
                "Looks wrong.",
                "@@ -1,3 +1,4 @@\n+new line\n old line",
            )],
            Some(10),
            Some(8),
        );
        print_thread(&thread, true);
    }

    #[test]
    fn print_thread_with_replies_does_not_panic() {
        let thread = make_thread(
            vec![
                make_comment("alice", "Initial comment.", ""),
                make_comment("bob", "Reply here.", ""),
            ],
            Some(5),
            None,
        );
        print_thread(&thread, false);
    }

    #[test]
    fn print_thread_line_range_does_not_panic() {
        let thread = make_thread(
            vec![make_comment("alice", "Range comment.", "")],
            Some(20),
            Some(15),
        );
        print_thread(&thread, false);
    }

    #[test]
    fn print_thread_no_line_does_not_panic() {
        let thread = make_thread(vec![make_comment("alice", "No line.", "")], None, None);
        print_thread(&thread, false);
    }

    // -------------------------------------------------------------------------
    // section_header — verify no panic
    // -------------------------------------------------------------------------

    #[test]
    fn section_header_does_not_panic() {
        section_header("Open PRs");
    }
}
