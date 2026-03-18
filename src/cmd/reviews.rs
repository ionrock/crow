use std::collections::BTreeMap;

use anyhow::Result;

use crate::display;
use crate::gh;

pub fn run(pr: Option<u64>, all: bool, diff: bool, _unresolved: bool) -> Result<()> {
    let pr_number = match pr {
        Some(n) => n,
        None => gh::current_pr_number()?,
    };

    let repo = gh::repo_info()?;
    let threads = gh::review_threads(repo.owner_login(), &repo.name, pr_number)?;

    // Filter threads
    let threads: Vec<_> = if all {
        threads
    } else {
        threads.into_iter().filter(|t| !t.is_resolved).collect()
    };

    if threads.is_empty() {
        if all {
            println!("No review threads on PR #{}.", pr_number);
        } else {
            println!("No unresolved review threads on PR #{}.", pr_number);
        }
        return Ok(());
    }

    // Group by file path
    let mut by_file: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for thread in threads {
        by_file.entry(thread.path.clone()).or_default().push(thread);
    }

    // Sort threads within each file by line number
    for threads in by_file.values_mut() {
        threads.sort_by_key(|t| t.line.unwrap_or(0));
    }

    for (path, threads) in &by_file {
        let unresolved_count = threads.iter().filter(|t| !t.is_resolved).count();
        display::section_header(&format!("{} ({} unresolved)", path, unresolved_count));

        for thread in threads {
            display::print_thread(thread, diff);
        }
    }

    Ok(())
}
