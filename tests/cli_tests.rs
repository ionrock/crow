use clap::Parser;
use crow::cli::{Cli, Command, ReviewEvent};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("crow").chain(args.iter().copied()))
}

// --- Status ---

#[test]
fn test_status_parses() {
    let cli = parse(&["status"]).expect("status should parse");
    assert!(matches!(cli.command, Command::Status));
}

// --- Checkout ---

#[test]
fn test_checkout_parses_pr_number() {
    let cli = parse(&["checkout", "42"]).expect("checkout 42 should parse");
    assert!(matches!(cli.command, Command::Checkout { pr: 42 }));
}

#[test]
fn test_checkout_missing_pr_errors() {
    let err = parse(&["checkout"]).expect_err("checkout without PR should fail");
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
}

// --- Reviews ---

#[test]
fn test_reviews_parses_no_args() {
    let cli = parse(&["reviews"]).expect("reviews should parse without args");
    match cli.command {
        Command::Reviews {
            pr,
            all,
            diff,
            unresolved,
        } => {
            assert_eq!(pr, None);
            assert!(!all);
            assert!(!diff);
            assert!(unresolved);
        }
        _ => panic!("expected Reviews"),
    }
}

#[test]
fn test_reviews_parses_all_flags() {
    let cli =
        parse(&["reviews", "42", "--all", "--diff"]).expect("reviews 42 --all --diff should parse");
    match cli.command {
        Command::Reviews { pr, all, diff, .. } => {
            assert_eq!(pr, Some(42));
            assert!(all);
            assert!(diff);
        }
        _ => panic!("expected Reviews"),
    }
}

// --- CI ---

#[test]
fn test_ci_parses_defaults() {
    let cli = parse(&["ci"]).expect("ci should parse without args");
    match cli.command {
        Command::Ci { pr, watch } => {
            assert_eq!(pr, None);
            assert!(!watch);
        }
        _ => panic!("expected Ci"),
    }
}

#[test]
fn test_ci_parses_with_watch() {
    let cli = parse(&["ci", "42", "--watch"]).expect("ci 42 --watch should parse");
    match cli.command {
        Command::Ci { pr, watch } => {
            assert_eq!(pr, Some(42));
            assert!(watch);
        }
        _ => panic!("expected Ci"),
    }
}

// --- Push ---

#[test]
fn test_push_parses_without_reply() {
    let cli = parse(&["push"]).expect("push should parse without --reply");
    match cli.command {
        Command::Push { reply } => {
            assert_eq!(reply, None);
        }
        _ => panic!("expected Push"),
    }
}

#[test]
fn test_push_parses_with_reply() {
    let cli = parse(&["push", "--reply", "Done"]).expect("push --reply Done should parse");
    match cli.command {
        Command::Push { reply } => {
            assert_eq!(reply.as_deref(), Some("Done"));
        }
        _ => panic!("expected Push"),
    }
}

// --- Done ---

#[test]
fn test_done_parses_without_ready() {
    let cli = parse(&["done"]).expect("done should parse without --ready");
    match cli.command {
        Command::Done { ready } => {
            assert!(!ready);
        }
        _ => panic!("expected Done"),
    }
}

#[test]
fn test_done_parses_with_ready() {
    let cli = parse(&["done", "--ready"]).expect("done --ready should parse");
    match cli.command {
        Command::Done { ready } => {
            assert!(ready);
        }
        _ => panic!("expected Done"),
    }
}

// --- Review ---

#[test]
fn test_review_parses_pr_number() {
    let cli = parse(&["review", "42"]).expect("review 42 should parse");
    assert!(matches!(cli.command, Command::Review { pr: 42 }));
}

// --- InstallPlugin ---

#[test]
fn test_install_plugin_parses_without_uninstall() {
    let cli = parse(&["install-plugin"]).expect("install-plugin should parse");
    match cli.command {
        Command::InstallPlugin { uninstall } => {
            assert!(!uninstall);
        }
        _ => panic!("expected InstallPlugin"),
    }
}

#[test]
fn test_install_plugin_parses_with_uninstall() {
    let cli =
        parse(&["install-plugin", "--uninstall"]).expect("install-plugin --uninstall should parse");
    match cli.command {
        Command::InstallPlugin { uninstall } => {
            assert!(uninstall);
        }
        _ => panic!("expected InstallPlugin"),
    }
}

// --- Comment ---

#[test]
fn test_comment_parses_all_fields() {
    let cli = parse(&["comment", "42", "--event", "approve", "hello"])
        .expect("comment 42 --event approve hello should parse");
    match cli.command {
        Command::Comment { pr, event, body } => {
            assert_eq!(pr, 42);
            assert!(matches!(event, ReviewEvent::Approve));
            assert_eq!(body.as_deref(), Some("hello"));
        }
        _ => panic!("expected Comment"),
    }
}

// --- Error cases ---

#[test]
fn test_invalid_command_errors() {
    let err = parse(&["notacommand"]).expect_err("invalid command should fail");
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
}
