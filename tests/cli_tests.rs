use clap::Parser;
use crow::cli::{Cli, Command};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("crow").chain(args.iter().copied()))
}

// --- Status ---

#[test]
fn test_status_parses() {
    let cli = parse(&["status"]).expect("status should parse");
    assert!(matches!(cli.command, Command::Status));
}

// --- Review ---

#[test]
fn test_review_parses_pr_number() {
    let cli = parse(&["review", "42"]).expect("review 42 should parse");
    assert!(matches!(cli.command, Command::Review { pr: 42 }));
}

#[test]
fn test_review_missing_pr_errors() {
    let err = parse(&["review"]).expect_err("review without PR should fail");
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
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

// --- Error cases ---

#[test]
fn test_invalid_command_errors() {
    let err = parse(&["notacommand"]).expect_err("invalid command should fail");
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
}
