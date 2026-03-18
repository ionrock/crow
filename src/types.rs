use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct ThreadComments {
    pub nodes: Vec<ThreadComment>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct WorkflowInfo {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct RepoInfo {
    pub owner: OwnerInfo,
    pub name: String,
}

impl RepoInfo {
    pub fn owner_login(&self) -> &str {
        &self.owner.login
    }
}

#[derive(Debug, Deserialize)]
pub struct OwnerInfo {
    pub login: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrFile {
    pub path: String,
    pub additions: u64,
    pub deletions: u64,
}
