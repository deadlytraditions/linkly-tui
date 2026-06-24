//! Serde data models mirroring the Linkly API schemas (see api-1.json).

use serde::{Deserialize, Serialize};

/// A link with click statistics, as returned by the workspace list endpoint.
/// `url`/`full_url` are part of the API payload and kept for completeness even
/// though the table view does not currently render them.
#[derive(Debug, Clone, Deserialize)]
pub struct LinkSummary {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub domain: Option<String>,
    #[allow(dead_code)]
    pub url: Option<String>,
    #[allow(dead_code)]
    pub full_url: Option<String>,
    pub enabled: Option<bool>,
    #[serde(default)]
    pub clicks_today: i64,
    #[serde(default)]
    pub clicks_thirty_days: i64,
    #[serde(default)]
    pub clicks_total: i64,
}

/// Paginated response from `GET /api/v1/workspace/{id}/list_links`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListLinksResponse {
    #[serde(default)]
    pub links: Vec<LinkSummary>,
    #[serde(default)]
    pub page_number: i64,
    #[serde(default)]
    #[allow(dead_code)]
    pub page_size: i64,
    #[serde(default)]
    pub total_pages: i64,
    #[serde(default)]
    pub total_entries: i64,
}

/// POST body for `POST /api/v1/link`. Only `workspace_id` and `url` are always
/// sent; every other field is omitted when `None` so we never overwrite server
/// defaults. This struct is the shared contract that the future CSV importer
/// will also build and submit.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateLinkRequest {
    pub workspace_id: i64,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_params: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloaking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_bots: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_referrer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_campaign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_term: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fb_pixel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiktok_pixel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gtm_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ga4_tag_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkify_words: Option<String>,
}

/// Response wrapper for `GET /api/v1/workspace/{id}/domains`.
#[derive(Debug, Clone, Deserialize)]
pub struct DomainList {
    #[serde(default)]
    pub domains: Vec<Domain>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Domain {
    pub name: String,
}

/// An entry from `GET /api/v1/workspaces`.
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: i64,
    #[serde(default)]
    pub name: String,
}
