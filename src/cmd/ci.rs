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
