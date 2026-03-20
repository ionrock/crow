// gh.rs — adapter for all `gh` CLI and GraphQL calls

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::process::Command;

use crate::types::{CheckRun, Pr, PrDetail, RepoInfo, ReviewThread, ThreadComment, ThreadComments};

// ---------------------------------------------------------------------------
// GhClient trait — injectable for testing
// ---------------------------------------------------------------------------

pub trait GhClient {
    fn current_pr_number(&self) -> Result<u64>;
    fn pr_list_authored(&self) -> Result<Vec<Pr>>;
    fn pr_list_review_requested(&self) -> Result<Vec<Pr>>;
    fn pr_checks(&self, pr: u64) -> Result<Vec<CheckRun>>;
    fn review_threads(&self, owner: &str, repo: &str, pr: u64) -> Result<Vec<ReviewThread>>;
    fn repo_info(&self) -> Result<RepoInfo>;
    fn reply_to_thread(
        &self,
        owner: &str,
        repo: &str,
        pr: u64,
        comment_id: &str,
        body: &str,
    ) -> Result<()>;
    fn current_user(&self) -> Result<String>;
    fn pr_author(&self, pr: u64) -> Result<String>;
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()>;
    fn pr_view(&self, pr: u64) -> Result<PrDetail>;
    fn pr_diff(&self, pr: u64) -> Result<String>;
    fn mark_ready(&self, pr: u64) -> Result<()>;
}

// ---------------------------------------------------------------------------
// Real implementation
// ---------------------------------------------------------------------------

pub struct RealGhClient;

impl GhClient for RealGhClient {
    fn current_pr_number(&self) -> Result<u64> {
        current_pr_number()
    }
    fn pr_list_authored(&self) -> Result<Vec<Pr>> {
        pr_list_authored()
    }
    fn pr_list_review_requested(&self) -> Result<Vec<Pr>> {
        pr_list_review_requested()
    }
    fn pr_checks(&self, pr: u64) -> Result<Vec<CheckRun>> {
        pr_checks(pr)
    }
    fn review_threads(&self, owner: &str, repo: &str, pr: u64) -> Result<Vec<ReviewThread>> {
        review_threads(owner, repo, pr)
    }
    fn repo_info(&self) -> Result<RepoInfo> {
        repo_info()
    }
    fn reply_to_thread(
        &self,
        owner: &str,
        repo: &str,
        pr: u64,
        comment_id: &str,
        body: &str,
    ) -> Result<()> {
        reply_to_thread(owner, repo, pr, comment_id, body)
    }
    fn current_user(&self) -> Result<String> {
        current_user()
    }
    fn pr_author(&self, pr: u64) -> Result<String> {
        pr_author(pr)
    }
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        post_review(pr, event, body)
    }
    fn pr_view(&self, pr: u64) -> Result<PrDetail> {
        pr_view(pr)
    }
    fn pr_diff(&self, pr: u64) -> Result<String> {
        pr_diff(pr)
    }
    fn mark_ready(&self, pr: u64) -> Result<()> {
        mark_ready(pr)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn run_gh(args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("Failed to run gh — is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh command failed: {}", stderr.trim());
    }

    Ok(output.stdout)
}

fn gh_json<T: DeserializeOwned>(args: &[&str]) -> Result<T> {
    let bytes = run_gh(args)?;
    serde_json::from_slice(&bytes).context("Failed to parse gh JSON output")
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn current_pr_number() -> Result<u64> {
    #[derive(Deserialize)]
    struct PrNumber {
        number: u64,
    }

    let pr: PrNumber = gh_json(&["pr", "view", "--json", "number"])
        .context("Not on a PR branch — run this from a branch associated with a pull request")?;

    Ok(pr.number)
}

pub fn pr_list_authored() -> Result<Vec<Pr>> {
    gh_json(&[
        "pr",
        "list",
        "--author",
        "@me",
        "--json",
        "number,title,headRefName,reviewDecision,updatedAt,url",
    ])
    .context("Failed to list authored PRs")
}

pub fn pr_list_review_requested() -> Result<Vec<Pr>> {
    gh_json(&[
        "pr",
        "list",
        "--search",
        "review-requested:@me",
        "--json",
        "number,title,headRefName,author,updatedAt,url",
    ])
    .context("Failed to list PRs with review requested")
}

pub fn pr_checks(pr: u64) -> Result<Vec<CheckRun>> {
    let pr_str = pr.to_string();
    gh_json(&[
        "pr",
        "checks",
        &pr_str,
        "--json",
        "name,state,bucket,description,workflow,completedAt,link",
    ])
    .context("Failed to fetch PR checks")
}

// ---------------------------------------------------------------------------
// GraphQL — review threads
// ---------------------------------------------------------------------------

// Private intermediate structs for deserializing the GraphQL response.

#[derive(Deserialize)]
struct GraphQLResponse {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    repository: Repository,
}

#[derive(Deserialize)]
struct Repository {
    #[serde(rename = "pullRequest")]
    pull_request: PullRequestData,
}

#[derive(Deserialize)]
struct PullRequestData {
    #[serde(rename = "reviewThreads")]
    review_threads: ReviewThreadConnection,
}

#[derive(Deserialize)]
struct ReviewThreadConnection {
    nodes: Vec<ReviewThreadNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadNode {
    id: String,
    is_resolved: bool,
    is_outdated: bool,
    path: String,
    line: Option<u64>,
    start_line: Option<u64>,
    comments: CommentConnection,
}

#[derive(Deserialize)]
struct CommentConnection {
    nodes: Vec<CommentNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentNode {
    id: String,
    author: AuthorNode,
    body: String,
    created_at: String,
    url: String,
    diff_hunk: String,
}

#[derive(Deserialize)]
struct AuthorNode {
    login: String,
}

const REVIEW_THREADS_QUERY: &str = r#"query($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      reviewThreads(first: 100) {
        nodes {
          id
          isResolved
          isOutdated
          path
          line
          startLine
          comments(first: 50) {
            nodes {
              id
              author { login }
              body
              createdAt
              url
              diffHunk
            }
          }
        }
      }
    }
  }
}"#;

pub fn review_threads(owner: &str, repo: &str, pr: u64) -> Result<Vec<ReviewThread>> {
    let pr_str = pr.to_string();
    let bytes = run_gh(&[
        "api",
        "graphql",
        "-f",
        &format!("query={}", REVIEW_THREADS_QUERY),
        "-f",
        &format!("owner={}", owner),
        "-f",
        &format!("repo={}", repo),
        "-F",
        &format!("number={}", pr_str),
    ])
    .context("Failed to fetch review threads")?;

    let response: GraphQLResponse =
        serde_json::from_slice(&bytes).context("Failed to parse GraphQL response")?;

    let threads = response
        .data
        .repository
        .pull_request
        .review_threads
        .nodes
        .into_iter()
        .map(|node| {
            let comments = node
                .comments
                .nodes
                .into_iter()
                .map(|c| ThreadComment {
                    id: c.id,
                    author: crate::types::Author {
                        login: c.author.login,
                    },
                    body: c.body,
                    created_at: c.created_at,
                    url: c.url,
                    diff_hunk: c.diff_hunk,
                })
                .collect();

            ReviewThread {
                id: node.id,
                is_resolved: node.is_resolved,
                is_outdated: node.is_outdated,
                path: node.path,
                line: node.line,
                start_line: node.start_line,
                comments: ThreadComments { nodes: comments },
            }
        })
        .collect();

    Ok(threads)
}

// ---------------------------------------------------------------------------
// Repo info
// ---------------------------------------------------------------------------

pub fn repo_info() -> Result<RepoInfo> {
    // `gh repo view --json owner,name` returns:
    //   { "owner": { "login": "..." }, "name": "..." }
    // RepoInfo already mirrors this structure (owner: OwnerInfo { login }),
    // so we can deserialize directly.
    gh_json(&["repo", "view", "--json", "owner,name"]).context("Failed to fetch repo info")
}

// ---------------------------------------------------------------------------
// Reply to a review thread
// ---------------------------------------------------------------------------

pub fn reply_to_thread(
    owner: &str,
    repo: &str,
    pr: u64,
    comment_id: &str,
    body: &str,
) -> Result<()> {
    let endpoint = format!("repos/{}/{}/pulls/{}/comments", owner, repo, pr);
    run_gh(&[
        "api",
        &endpoint,
        "-f",
        &format!("body={}", body),
        "-F",
        &format!("in_reply_to={}", comment_id),
    ])
    .context("Failed to reply to review thread")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Current authenticated user
// ---------------------------------------------------------------------------

pub fn current_user() -> Result<String> {
    let bytes =
        run_gh(&["api", "user", "--jq", ".login"]).context("Failed to fetch current user")?;
    let login = String::from_utf8(bytes)
        .context("gh returned non-UTF-8 output for user login")?
        .trim()
        .to_string();
    Ok(login)
}

// ---------------------------------------------------------------------------
// PR author login
// ---------------------------------------------------------------------------

pub fn pr_author(pr: u64) -> Result<String> {
    let pr_str = pr.to_string();
    let bytes = run_gh(&[
        "pr",
        "view",
        &pr_str,
        "--json",
        "author",
        "--jq",
        ".author.login",
    ])
    .context("Failed to fetch PR author")?;
    let login = String::from_utf8(bytes)
        .context("gh returned non-UTF-8 output for PR author")?
        .trim()
        .to_string();
    Ok(login)
}

// ---------------------------------------------------------------------------
// Post a review
// ---------------------------------------------------------------------------

pub fn post_review(pr: u64, event: &str, body: &str) -> Result<()> {
    let pr_str = pr.to_string();
    let event_flag = format!("--{}", event);
    run_gh(&["pr", "review", &pr_str, &event_flag, "--body", body])
        .context("Failed to post review")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// PR description and diff for review context
// ---------------------------------------------------------------------------

pub fn pr_view(pr: u64) -> Result<PrDetail> {
    let pr_str = pr.to_string();
    gh_json(&[
        "pr",
        "view",
        &pr_str,
        "--json",
        "number,title,body,headRefName,baseRefName,author,url,files",
    ])
    .context("Failed to fetch PR details")
}

pub fn pr_diff(pr: u64) -> Result<String> {
    let pr_str = pr.to_string();
    let bytes = run_gh(&["pr", "diff", &pr_str]).context("Failed to fetch PR diff")?;
    String::from_utf8(bytes).context("PR diff is not valid UTF-8")
}

// ---------------------------------------------------------------------------
// Mark PR as ready for review
// ---------------------------------------------------------------------------

pub fn mark_ready(pr: u64) -> Result<()> {
    let pr_str = pr.to_string();
    run_gh(&["pr", "ready", &pr_str]).context("Failed to mark PR as ready")?;
    Ok(())
}
