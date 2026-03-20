use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pr {
    pub number: u64,
    pub title: String,
    pub head_ref_name: String,
    pub review_decision: Option<String>,
    pub updated_at: String,
    pub url: String,
    pub author: Option<Author>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewThread {
    pub id: String,
    pub is_resolved: bool,
    pub is_outdated: bool,
    pub path: String,
    pub line: Option<u64>,
    pub start_line: Option<u64>,
    pub comments: ThreadComments,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThreadComments {
    pub nodes: Vec<ThreadComment>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadComment {
    pub id: String,
    pub author: Author,
    pub body: String,
    pub created_at: String,
    pub url: String,
    pub diff_hunk: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRun {
    pub name: String,
    pub state: String,
    pub bucket: String,
    pub description: Option<String>,
    pub workflow: WorkflowInfo,
    pub completed_at: Option<String>,
    pub link: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowInfo {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoInfo {
    pub owner: OwnerInfo,
    pub name: String,
}

impl RepoInfo {
    pub fn owner_login(&self) -> &str {
        &self.owner.login
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OwnerInfo {
    pub login: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrDetail {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub head_ref_name: String,
    pub base_ref_name: String,
    pub author: Author,
    pub url: String,
    pub files: Vec<PrFile>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrFile {
    pub path: String,
    pub additions: u64,
    pub deletions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pr ---

    #[test]
    fn test_pr_deserialize_all_fields() {
        let json = r#"{
            "number": 42,
            "title": "Fix everything",
            "headRefName": "feat/fix",
            "reviewDecision": "APPROVED",
            "updatedAt": "2024-01-01T00:00:00Z",
            "url": "https://github.com/owner/repo/pull/42",
            "author": { "login": "alice" }
        }"#;
        let pr: Pr = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Fix everything");
        assert_eq!(pr.head_ref_name, "feat/fix");
        assert_eq!(pr.review_decision.as_deref(), Some("APPROVED"));
        assert_eq!(pr.author.as_ref().map(|a| a.login.as_str()), Some("alice"));
    }

    #[test]
    fn test_pr_deserialize_optional_fields_null() {
        let json = r#"{
            "number": 1,
            "title": "Draft PR",
            "headRefName": "main",
            "reviewDecision": null,
            "updatedAt": "2024-06-01T12:00:00Z",
            "url": "https://github.com/owner/repo/pull/1",
            "author": null
        }"#;
        let pr: Pr = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 1);
        assert!(pr.review_decision.is_none());
        assert!(pr.author.is_none());
    }

    // --- ReviewThread ---

    #[test]
    fn test_review_thread_unresolved_with_line_range() {
        let json = r#"{
            "id": "thread-1",
            "isResolved": false,
            "isOutdated": false,
            "path": "src/main.rs",
            "line": 20,
            "startLine": 15,
            "comments": {
                "nodes": [
                    {
                        "id": "c1",
                        "author": { "login": "bob" },
                        "body": "Consider refactoring",
                        "createdAt": "2024-01-02T00:00:00Z",
                        "url": "https://github.com/owner/repo/pull/1#comment-c1",
                        "diffHunk": "@@ -10,6 +10,6 @@"
                    }
                ]
            }
        }"#;
        let thread: ReviewThread = serde_json::from_str(json).unwrap();
        assert!(!thread.is_resolved);
        assert_eq!(thread.path, "src/main.rs");
        assert_eq!(thread.line, Some(20));
        assert_eq!(thread.start_line, Some(15));
        assert_eq!(thread.comments.nodes.len(), 1);
        assert_eq!(thread.comments.nodes[0].author.login, "bob");
    }

    #[test]
    fn test_review_thread_resolved_no_line_numbers() {
        let json = r#"{
            "id": "thread-2",
            "isResolved": true,
            "isOutdated": true,
            "path": "README.md",
            "line": null,
            "startLine": null,
            "comments": { "nodes": [] }
        }"#;
        let thread: ReviewThread = serde_json::from_str(json).unwrap();
        assert!(thread.is_resolved);
        assert!(thread.is_outdated);
        assert!(thread.line.is_none());
        assert!(thread.start_line.is_none());
        assert!(thread.comments.nodes.is_empty());
    }

    // --- ThreadComment ---

    #[test]
    fn test_thread_comment_all_fields() {
        let json = r#"{
            "id": "IC_abc123",
            "author": { "login": "carol" },
            "body": "This looks good to me.",
            "createdAt": "2024-03-10T08:00:00Z",
            "url": "https://github.com/owner/repo/pull/5#discussion_r1",
            "diffHunk": "@@ -1,3 +1,4 @@\n+new line"
        }"#;
        let comment: ThreadComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, "IC_abc123");
        assert_eq!(comment.author.login, "carol");
        assert_eq!(comment.body, "This looks good to me.");
        assert_eq!(comment.diff_hunk, "@@ -1,3 +1,4 @@\n+new line");
    }

    // --- CheckRun ---

    #[test]
    fn test_check_run_success_state() {
        let json = r#"{
            "name": "CI / test",
            "state": "SUCCESS",
            "bucket": "pass",
            "description": "All tests passed",
            "workflow": { "name": "CI" },
            "completedAt": "2024-01-01T01:00:00Z",
            "link": "https://github.com/owner/repo/actions/runs/1"
        }"#;
        let run: CheckRun = serde_json::from_str(json).unwrap();
        assert_eq!(run.state, "SUCCESS");
        assert_eq!(run.description.as_deref(), Some("All tests passed"));
        assert_eq!(run.workflow.name, "CI");
    }

    #[test]
    fn test_check_run_failure_no_optional_fields() {
        let json = r#"{
            "name": "lint",
            "state": "FAILURE",
            "bucket": "fail",
            "description": null,
            "workflow": { "name": "Lint" },
            "completedAt": null,
            "link": "https://github.com/owner/repo/actions/runs/2"
        }"#;
        let run: CheckRun = serde_json::from_str(json).unwrap();
        assert_eq!(run.state, "FAILURE");
        assert!(run.description.is_none());
        assert!(run.completed_at.is_none());
    }

    #[test]
    fn test_check_run_pending_state() {
        let json = r#"{
            "name": "build",
            "state": "PENDING",
            "bucket": "pending",
            "description": null,
            "workflow": { "name": "Build" },
            "completedAt": null,
            "link": "https://github.com/owner/repo/actions/runs/3"
        }"#;
        let run: CheckRun = serde_json::from_str(json).unwrap();
        assert_eq!(run.state, "PENDING");
    }

    // --- RepoInfo ---

    #[test]
    fn test_repo_info_owner_login_accessor() {
        let json = r#"{
            "owner": { "login": "my-org" },
            "name": "my-repo"
        }"#;
        let repo: RepoInfo = serde_json::from_str(json).unwrap();
        assert_eq!(repo.owner_login(), "my-org");
        assert_eq!(repo.name, "my-repo");
    }

    // --- PrDetail ---

    #[test]
    fn test_pr_detail_with_files() {
        let json = r#"{
            "number": 7,
            "title": "Add feature",
            "body": "This PR adds a great feature.",
            "headRefName": "feat/new",
            "baseRefName": "main",
            "author": { "login": "dave" },
            "url": "https://github.com/owner/repo/pull/7",
            "files": [
                { "path": "src/lib.rs", "additions": 50, "deletions": 10 },
                { "path": "tests/lib_test.rs", "additions": 30, "deletions": 0 }
            ]
        }"#;
        let detail: PrDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.number, 7);
        assert_eq!(detail.body, "This PR adds a great feature.");
        assert_eq!(detail.files.len(), 2);
        assert_eq!(detail.files[0].path, "src/lib.rs");
        assert_eq!(detail.files[1].deletions, 0);
    }

    #[test]
    fn test_pr_detail_empty_body() {
        let json = r#"{
            "number": 8,
            "title": "Minor tweak",
            "body": "",
            "headRefName": "tweak",
            "baseRefName": "main",
            "author": { "login": "eve" },
            "url": "https://github.com/owner/repo/pull/8",
            "files": []
        }"#;
        let detail: PrDetail = serde_json::from_str(json).unwrap();
        assert!(detail.body.is_empty());
        assert!(detail.files.is_empty());
    }

    // --- PrFile ---

    #[test]
    fn test_pr_file_additions_deletions() {
        let json = r#"{
            "path": "src/cmd/review.rs",
            "additions": 120,
            "deletions": 45
        }"#;
        let file: PrFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.path, "src/cmd/review.rs");
        assert_eq!(file.additions, 120);
        assert_eq!(file.deletions, 45);
    }
}
