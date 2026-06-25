//! Thin async HTTP client for the Linkly API.
//!
//! Authentication uses the `api_key` query parameter, which is injected into
//! every request. The client is cheaply `Clone`-able so it can be moved into
//! spawned tokio tasks.

use anyhow::{bail, Result};
use serde_json::Value;

use super::models::{ClicksResponse, CreateLinkRequest, DomainList, ListLinksResponse, Workspace};

const BASE_URL: &str = "https://api.linklyhq.com";

#[derive(Clone)]
pub struct LinklyClient {
    http: reqwest::Client,
    api_key: String,
}

impl LinklyClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
        }
    }

    /// List links in a workspace, paginated and sorted.
    pub async fn list_links(
        &self,
        workspace_id: i64,
        page: i64,
        page_size: i64,
        search: &str,
        sort_by: &str,
        sort_dir: &str,
    ) -> Result<ListLinksResponse> {
        let url = format!("{BASE_URL}/api/v1/workspace/{workspace_id}/list_links");
        let mut req = self.http.get(url).query(&[
            ("api_key", self.api_key.clone()),
            ("page", page.to_string()),
            ("page_size", page_size.to_string()),
            ("sort_by", sort_by.to_string()),
            ("sort_dir", sort_dir.to_string()),
        ]);
        if !search.is_empty() {
            req = req.query(&[("search", search)]);
        }
        let resp = check(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    /// Fetch the full detail record for a single link as a JSON object.
    pub async fn get_link(&self, id: i64, workspace_id: i64) -> Result<Value> {
        let url = format!("{BASE_URL}/api/v1/link/{id}");
        let req = self.http.get(url).query(&[
            ("api_key", self.api_key.clone()),
            ("workspace_id", workspace_id.to_string()),
        ]);
        let resp = check(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    /// Create (or update, if `id` were set) a link.
    pub async fn create_link(&self, body: &CreateLinkRequest) -> Result<Value> {
        let url = format!("{BASE_URL}/api/v1/link");
        let req = self
            .http
            .post(url)
            .query(&[("api_key", self.api_key.clone())])
            .json(body);
        let resp = check(req.send().await?).await?;
        Ok(resp.json().await.unwrap_or(Value::Null))
    }

    /// Update an existing link. `body` must contain `id` and `workspace_id`
    /// plus the fields to change. Posts to the same endpoint as create.
    pub async fn update_link(&self, body: Value) -> Result<Value> {
        let url = format!("{BASE_URL}/api/v1/link");
        let req = self
            .http
            .post(url)
            .query(&[("api_key", self.api_key.clone())])
            .json(&body);
        let resp = check(req.send().await?).await?;
        Ok(resp.json().await.unwrap_or(Value::Null))
    }

    /// Daily click counts for a single link between `start` and `end`
    /// (`YYYY-MM-DD`). Returns `(date, count)` pairs.
    pub async fn get_clicks(
        &self,
        workspace_id: i64,
        link_id: i64,
        start: &str,
        end: &str,
    ) -> Result<Vec<(String, i64)>> {
        let url = format!("{BASE_URL}/api/v1/workspace/{workspace_id}/clicks");
        let req = self.http.get(url).query(&[
            ("api_key", self.api_key.clone()),
            ("link_id", link_id.to_string()),
            ("start", start.to_string()),
            ("end", end.to_string()),
            ("frequency", "day".to_string()),
        ]);
        let resp = check(req.send().await?).await?;
        let parsed: ClicksResponse = resp.json().await?;
        Ok(parsed
            .traffic
            .into_iter()
            .map(|p| (p.t.unwrap_or_default(), p.y.unwrap_or(0)))
            .collect())
    }

    /// List the workspaces the API key can access.
    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let url = format!("{BASE_URL}/api/v1/workspaces");
        let req = self
            .http
            .get(url)
            .query(&[("api_key", self.api_key.clone())]);
        let resp = check(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    /// List the custom domains available in a workspace.
    pub async fn list_domains(&self, workspace_id: i64) -> Result<Vec<String>> {
        let url = format!("{BASE_URL}/api/v1/workspace/{workspace_id}/domains");
        let req = self
            .http
            .get(url)
            .query(&[("api_key", self.api_key.clone())]);
        let resp = check(req.send().await?).await?;
        let list: DomainList = resp.json().await?;
        Ok(list.domains.into_iter().map(|d| d.name).collect())
    }
}

/// Turn a non-2xx response into an error carrying the status and body.
async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
    if resp.status().is_success() {
        Ok(resp)
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let body = body.trim();
        if body.is_empty() {
            bail!("HTTP {status}")
        } else {
            bail!("HTTP {status}: {body}")
        }
    }
}
