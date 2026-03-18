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
