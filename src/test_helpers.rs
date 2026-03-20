// test_helpers.rs — shared mock implementations for unit tests

use std::cell::{Cell, RefCell};

use anyhow::Result;

use crate::gh::GhClient;
use crate::types::{Author, CheckRun, OwnerInfo, Pr, PrDetail, PrFile, RepoInfo, ReviewThread};
use crate::wt::WtClient;

// ---------------------------------------------------------------------------
// Recorded reply call
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct RecordedReply {
    pub owner: String,
    pub repo: String,
    pub pr: u64,
    pub comment_id: String,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Recorded review call
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct RecordedReview {
    pub pr: u64,
    pub event: String,
    pub body: String,
}

// ---------------------------------------------------------------------------
// MockGhClient
// ---------------------------------------------------------------------------

pub struct MockGhClient {
    pub current_pr: u64,
    pub authored: Vec<Pr>,
    pub review_requested: Vec<Pr>,
    pub checks: Vec<CheckRun>,
    pub threads: Vec<ReviewThread>,
    pub pr_author_login: String,
    pub current_user_login: String,
    pub mark_ready_called: Cell<bool>,
    pub reviews: RefCell<Vec<RecordedReview>>,
    pub replies: RefCell<Vec<RecordedReply>>,
}

impl MockGhClient {
    pub fn new() -> Self {
        Self {
            current_pr: 1,
            authored: vec![],
            review_requested: vec![],
            checks: vec![],
            threads: vec![],
            pr_author_login: "author".to_string(),
            current_user_login: "author".to_string(),
            mark_ready_called: Cell::new(false),
            reviews: RefCell::new(vec![]),
            replies: RefCell::new(vec![]),
        }
    }
}

impl GhClient for MockGhClient {
    fn current_pr_number(&self) -> Result<u64> {
        Ok(self.current_pr)
    }

    fn pr_list_authored(&self) -> Result<Vec<Pr>> {
        Ok(self.authored.clone())
    }

    fn pr_list_review_requested(&self) -> Result<Vec<Pr>> {
        Ok(self.review_requested.clone())
    }

    fn pr_checks(&self, _pr: u64) -> Result<Vec<CheckRun>> {
        Ok(self.checks.clone())
    }

    fn review_threads(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<Vec<ReviewThread>> {
        Ok(self.threads.clone())
    }

    fn repo_info(&self) -> Result<RepoInfo> {
        Ok(RepoInfo {
            owner: OwnerInfo {
                login: "test-owner".to_string(),
            },
            name: "test-repo".to_string(),
        })
    }

    fn reply_to_thread(
        &self,
        owner: &str,
        repo: &str,
        pr: u64,
        comment_id: &str,
        body: &str,
    ) -> Result<()> {
        self.replies.borrow_mut().push(RecordedReply {
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr,
            comment_id: comment_id.to_string(),
            body: body.to_string(),
        });
        Ok(())
    }

    fn current_user(&self) -> Result<String> {
        Ok(self.current_user_login.clone())
    }

    fn pr_author(&self, _pr: u64) -> Result<String> {
        Ok(self.pr_author_login.clone())
    }

    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        self.reviews.borrow_mut().push(RecordedReview {
            pr,
            event: event.to_string(),
            body: body.to_string(),
        });
        Ok(())
    }

    fn pr_view(&self, pr: u64) -> Result<PrDetail> {
        Ok(PrDetail {
            number: pr,
            title: "Test PR".to_string(),
            body: "PR body".to_string(),
            head_ref_name: "feat/branch".to_string(),
            base_ref_name: "main".to_string(),
            author: Author {
                login: self.pr_author_login.clone(),
            },
            url: format!("https://github.com/test-owner/test-repo/pull/{}", pr),
            files: vec![PrFile {
                path: "src/main.rs".to_string(),
                additions: 10,
                deletions: 2,
            }],
        })
    }

    fn pr_diff(&self, _pr: u64) -> Result<String> {
        Ok("diff --git a/src/main.rs b/src/main.rs\n+new line\n".to_string())
    }

    fn mark_ready(&self, _pr: u64) -> Result<()> {
        self.mark_ready_called.set(true);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MockWtClient
// ---------------------------------------------------------------------------

pub struct MockWtClient {
    pub checked_out_pr: Cell<Option<u64>>,
    pub removed: Cell<bool>,
}

impl MockWtClient {
    pub fn new() -> Self {
        Self {
            checked_out_pr: Cell::new(None),
            removed: Cell::new(false),
        }
    }
}

impl WtClient for MockWtClient {
    fn checkout_pr(&self, pr: u64) -> Result<()> {
        self.checked_out_pr.set(Some(pr));
        Ok(())
    }

    fn checkout_pr_exec(&self, _pr: u64, _cmd: &str, _args: &[&str]) -> Result<()> {
        Ok(())
    }

    fn remove_current(&self) -> Result<()> {
        self.removed.set(true);
        Ok(())
    }
}
