use std::collections::BTreeMap;

use anyhow::Result;

use crate::display;
use crate::gh::GhClient;

pub fn run(gh: &dyn GhClient, pr: Option<u64>, watch: bool) -> Result<()> {
    let pr_number = match pr {
        Some(n) => n,
        None => gh.current_pr_number()?,
    };

    if watch {
        // Delegate entirely to `gh pr checks --watch`
        use std::os::unix::process::CommandExt;
        let pr_str = pr_number.to_string();
        let err = std::process::Command::new("gh")
            .args(["pr", "checks", &pr_str, "--watch"])
            .exec();
        // exec only returns on error
        anyhow::bail!("Failed to exec gh pr checks --watch: {}", err);
    }

    let checks = gh.pr_checks(pr_number)?;

    if checks.is_empty() {
        println!("No CI checks on PR #{}.", pr_number);
        return Ok(());
    }

    // Group by workflow name
    let mut by_workflow: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for check in checks {
        by_workflow
            .entry(check.workflow.name.clone())
            .or_default()
            .push(check);
    }

    for (workflow, checks) in &by_workflow {
        display::section_header(&format!("  {}", workflow));
        for check in checks {
            display::print_check_row(check);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockGhClient;
    use crate::types::{CheckRun, WorkflowInfo};

    fn make_check(name: &str, workflow: &str, state: &str) -> CheckRun {
        CheckRun {
            name: name.to_string(),
            state: state.to_string(),
            bucket: "pass".to_string(),
            description: None,
            workflow: WorkflowInfo {
                name: workflow.to_string(),
            },
            completed_at: Some("2024-01-01T00:00:00Z".to_string()),
            link: "https://github.com/actions/run/1".to_string(),
        }
    }

    #[test]
    fn no_checks_with_explicit_pr_number() {
        let mut mock = MockGhClient::new();
        mock.checks = vec![];
        run(&mock, Some(10), false).unwrap();
    }

    #[test]
    fn falls_back_to_current_pr_number() {
        let mut mock = MockGhClient::new();
        mock.current_pr = 7;
        mock.checks = vec![];
        run(&mock, None, false).unwrap();
    }

    #[test]
    fn multiple_checks_grouped_by_workflow() {
        let mut mock = MockGhClient::new();
        mock.checks = vec![
            make_check("test", "CI", "SUCCESS"),
            make_check("lint", "CI", "SUCCESS"),
            make_check("build", "Release", "SUCCESS"),
        ];
        run(&mock, Some(1), false).unwrap();
    }

    #[test]
    fn failed_check_shows_link() {
        let mut mock = MockGhClient::new();
        mock.checks = vec![make_check("test", "CI", "FAILURE")];
        run(&mock, Some(1), false).unwrap();
    }
}
