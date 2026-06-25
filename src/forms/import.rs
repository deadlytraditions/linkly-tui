//! CSV bulk-import: parsing, field mapping, file browser, and result types.
//!
//! Columns are link field names (header row). `url` is required; every other
//! field is optional and omitted when blank. Unknown columns are ignored with a
//! warning. Rows map to [`CreateLinkRequest`] — the same write contract used by
//! the single-create form — so submission reuses `LinklyClient::create_link`.

use std::path::{Path, PathBuf};

use anyhow::Result;
use ratatui::widgets::ListState;
use serde_json::Value;

use crate::api::models::CreateLinkRequest;

/// Every CSV-settable column, in template order. `workspace_id` is implicit
/// (the active workspace) and intentionally not importable.
pub const SUPPORTED: &[&str] = &[
    "url",
    "slug",
    "name",
    "domain",
    "note",
    "enabled",
    "forward_params",
    "cloaking",
    "block_bots",
    "hide_referrer",
    "og_title",
    "og_description",
    "og_image",
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "fb_pixel_id",
    "tiktok_pixel_id",
    "gtm_id",
    "ga4_tag_id",
    "head_tags",
    "body_tags",
    "linkify_words",
];

fn parse_bool(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" | "on" => Some(true),
        "false" | "0" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

fn set_str(field: &mut Option<String>, v: &str) {
    if !v.is_empty() {
        *field = Some(v.to_string());
    }
}

fn set_bool(field: &mut Option<bool>, v: &str) {
    if let Some(b) = parse_bool(v) {
        *field = Some(b);
    }
}

/// Apply one CSV cell to the request. Returns whether the header is a known
/// field. Blank values are ignored (the field stays unset/None).
pub fn set_field(req: &mut CreateLinkRequest, header: &str, value: &str) -> bool {
    let v = value.trim();
    match header.trim().to_ascii_lowercase().as_str() {
        "url" => req.url = v.to_string(),
        "slug" => set_str(&mut req.slug, v),
        "name" => set_str(&mut req.name, v),
        "domain" => set_str(&mut req.domain, v),
        "note" => set_str(&mut req.note, v),
        "og_title" => set_str(&mut req.og_title, v),
        "og_description" => set_str(&mut req.og_description, v),
        "og_image" => set_str(&mut req.og_image, v),
        "utm_source" => set_str(&mut req.utm_source, v),
        "utm_medium" => set_str(&mut req.utm_medium, v),
        "utm_campaign" => set_str(&mut req.utm_campaign, v),
        "utm_term" => set_str(&mut req.utm_term, v),
        "utm_content" => set_str(&mut req.utm_content, v),
        "fb_pixel_id" => set_str(&mut req.fb_pixel_id, v),
        "tiktok_pixel_id" => set_str(&mut req.tiktok_pixel_id, v),
        "gtm_id" => set_str(&mut req.gtm_id, v),
        "ga4_tag_id" => set_str(&mut req.ga4_tag_id, v),
        "head_tags" => set_str(&mut req.head_tags, v),
        "body_tags" => set_str(&mut req.body_tags, v),
        "linkify_words" => set_str(&mut req.linkify_words, v),
        "enabled" => set_bool(&mut req.enabled, v),
        "forward_params" => set_bool(&mut req.forward_params, v),
        "cloaking" => set_bool(&mut req.cloaking, v),
        "block_bots" => set_bool(&mut req.block_bots, v),
        "hide_referrer" => set_bool(&mut req.hide_referrer, v),
        _ => return false,
    }
    true
}

/// One parsed CSV row.
pub struct ParsedRow {
    pub line: u64,
    pub request: Option<CreateLinkRequest>,
    pub url_display: String,
    pub slug_display: String,
    pub error: Option<String>,
}

/// The result of parsing a CSV file.
pub struct ParsedImport {
    pub path: PathBuf,
    pub rows: Vec<ParsedRow>,
    pub warnings: Vec<String>,
}

impl ParsedImport {
    pub fn total(&self) -> usize {
        self.rows.len()
    }
    pub fn valid(&self) -> usize {
        self.rows.iter().filter(|r| r.error.is_none()).count()
    }
    pub fn invalid(&self) -> usize {
        self.total() - self.valid()
    }
    /// (line, request) pairs for the rows that can be submitted.
    pub fn valid_requests(&self) -> Vec<(u64, CreateLinkRequest)> {
        self.rows
            .iter()
            .filter_map(|r| r.request.clone().map(|req| (r.line, req)))
            .collect()
    }
}

/// Parse a CSV file into per-row [`CreateLinkRequest`]s for `workspace_id`.
pub fn parse_csv(path: &Path, workspace_id: i64) -> Result<ParsedImport> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?;
    let headers = rdr.headers()?.clone();

    let mut warnings = Vec::new();
    for h in headers.iter() {
        if !SUPPORTED.contains(&h.to_ascii_lowercase().as_str()) {
            warnings.push(format!("ignoring unknown column “{h}”"));
        }
    }

    let mut rows = Vec::new();
    for (idx, record) in rdr.records().enumerate() {
        let line = idx as u64 + 2; // header is line 1
        let record = match record {
            Ok(r) => r,
            Err(e) => {
                rows.push(ParsedRow {
                    line,
                    request: None,
                    url_display: String::new(),
                    slug_display: String::new(),
                    error: Some(format!("malformed row: {e}")),
                });
                continue;
            }
        };

        let mut req = CreateLinkRequest {
            workspace_id,
            ..Default::default()
        };
        for (i, val) in record.iter().enumerate() {
            if let Some(h) = headers.get(i) {
                set_field(&mut req, h, val);
            }
        }

        let url_display = req.url.clone();
        let slug_display = req.slug.clone().unwrap_or_default();
        let error = if req.url.trim().is_empty() {
            Some("missing url".to_string())
        } else {
            None
        };
        rows.push(ParsedRow {
            line,
            request: error.is_none().then_some(req),
            url_display,
            slug_display,
            error,
        });
    }

    Ok(ParsedImport {
        path: path.to_path_buf(),
        rows,
        warnings,
    })
}

fn example_value(field: &str) -> &'static str {
    match field {
        "url" => "https://example.com/landing",
        "slug" => "spring-promo",
        "name" => "Spring promo",
        "enabled" => "true",
        "utm_source" => "newsletter",
        "utm_medium" => "email",
        "utm_campaign" => "spring",
        _ => "",
    }
}

/// Write a template CSV (all supported headers + one example row).
pub fn write_template(path: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(SUPPORTED.iter().copied())?;
    let example: Vec<&str> = SUPPORTED.iter().map(|f| example_value(f)).collect();
    wtr.write_record(example)?;
    wtr.flush()?;
    Ok(())
}

/// A link created during import, parsed from the API's create response.
#[derive(Debug, Clone)]
pub struct NewLink {
    pub id: Option<i64>,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub full_url: Option<String>,
    pub url: Option<String>,
}

impl NewLink {
    pub fn from_response(v: &Value) -> Self {
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        Self {
            id: v.get("id").and_then(Value::as_i64),
            slug: s("slug"),
            name: s("name"),
            full_url: s("full_url"),
            url: s("url"),
        }
    }
}

/// Write the rows that were created successfully. Returns the file path.
pub fn write_success(dir: &Path, links: &[NewLink]) -> Result<PathBuf> {
    let path = dir.join("linkly-import-success.csv");
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record(["id", "slug", "name", "full_url", "url"])?;
    for l in links {
        wtr.write_record([
            l.id.map(|i| i.to_string()).unwrap_or_default(),
            l.slug.clone().unwrap_or_default(),
            l.name.clone().unwrap_or_default(),
            l.full_url.clone().unwrap_or_default(),
            l.url.clone().unwrap_or_default(),
        ])?;
    }
    wtr.flush()?;
    Ok(path)
}

/// Write the rows that failed (line number + reason). Returns the file path.
pub fn write_failures(dir: &Path, failures: &[(u64, String)]) -> Result<PathBuf> {
    let path = dir.join("linkly-import-failures.csv");
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record(["line", "reason"])?;
    for (line, reason) in failures {
        wtr.write_record([line.to_string(), reason.clone()])?;
    }
    wtr.flush()?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// File browser
// ---------------------------------------------------------------------------

/// Directories hidden from the browser (build/system/output clutter).
const IGNORED_DIRS: &[&str] = &["target", "src", "linkly-qr", "node_modules"];

pub struct BrowserEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct FileBrowser {
    pub dir: PathBuf,
    pub entries: Vec<BrowserEntry>,
    pub state: ListState,
}

impl FileBrowser {
    pub fn new() -> Self {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut browser = Self {
            dir,
            entries: Vec::new(),
            state: ListState::default(),
        };
        browser.refresh();
        browser
    }

    /// Rebuild the entry list for the current directory: a `..` parent, then
    /// sub-directories, then `.csv` files (each alphabetical).
    pub fn refresh(&mut self) {
        let mut entries = Vec::new();
        if let Some(parent) = self.dir.parent() {
            entries.push(BrowserEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            });
        }
        let (mut dirs, mut files) = (Vec::new(), Vec::new());
        if let Ok(read) = std::fs::read_dir(&self.dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let path = entry.path();
                if path.is_dir() {
                    if IGNORED_DIRS.contains(&name.as_str()) {
                        continue;
                    }
                    dirs.push(BrowserEntry { name, path, is_dir: true });
                } else if path
                    .extension()
                    .map(|x| x.eq_ignore_ascii_case("csv"))
                    .unwrap_or(false)
                {
                    files.push(BrowserEntry { name, path, is_dir: false });
                }
            }
        }
        dirs.sort_by_key(|e| e.name.to_lowercase());
        files.sort_by_key(|e| e.name.to_lowercase());
        entries.extend(dirs);
        entries.extend(files);
        self.entries = entries;
        self.state
            .select((!self.entries.is_empty()).then_some(0));
    }

    pub fn move_up(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(i.saturating_sub(1)));
    }

    pub fn move_down(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        self.state
            .select(Some((i + 1).min(self.entries.len().saturating_sub(1))));
    }

    pub fn selected(&self) -> Option<&BrowserEntry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    pub fn open_dir(&mut self, dir: PathBuf) {
        self.dir = dir;
        self.refresh();
    }
}

// ---------------------------------------------------------------------------
// Screen state
// ---------------------------------------------------------------------------

/// Live progress while submitting.
pub struct Progress {
    pub total: usize,
    pub done: usize,
    pub ok: usize,
    pub failed: usize,
}

/// Final summary after submission.
pub struct Summary {
    pub ok: usize,
    pub failed: usize,
    pub success_path: Option<PathBuf>,
    pub failure_path: Option<PathBuf>,
    pub new_links: Vec<NewLink>,
    /// `None` = awaiting the QR decision; `Some(n)` = generated n QR codes.
    pub qr_done: Option<usize>,
    pub qr_dir: Option<String>,
}

pub enum ImportStage {
    Browse,
    Preview(ParsedImport),
    Running(Progress),
    Done(Summary),
}

pub struct ImportState {
    pub browser: FileBrowser,
    pub stage: ImportStage,
    /// Last status/error line for the browser stage (e.g. parse error).
    pub message: Option<String>,
}

impl ImportState {
    pub fn new() -> Self {
        Self {
            browser: FileBrowser::new(),
            stage: ImportStage::Browse,
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_csv(contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("linkly-imp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}.csv", rand_name()));
        std::fs::File::create(&path)
            .unwrap()
            .write_all(contents.as_bytes())
            .unwrap();
        path
    }

    fn rand_name() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        format!("t{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())
    }

    #[test]
    fn parses_valid_invalid_and_unknown_columns() {
        let path = tmp_csv(
            "url,slug,enabled,bogus\nhttps://a.com,promo,yes,x\n,blank,no,y\nhttps://b.com,,,\n",
        );
        let parsed = parse_csv(&path, 42).unwrap();

        assert_eq!(parsed.total(), 3);
        assert_eq!(parsed.valid(), 2);
        assert_eq!(parsed.invalid(), 1);
        assert!(parsed.warnings.iter().any(|w| w.contains("bogus")));

        let reqs = parsed.valid_requests();
        // First valid row: url + slug + enabled=true.
        let (line, r0) = &reqs[0];
        assert_eq!(*line, 2);
        assert_eq!(r0.url, "https://a.com");
        assert_eq!(r0.slug.as_deref(), Some("promo"));
        assert_eq!(r0.enabled, Some(true));
        assert_eq!(r0.workspace_id, 42);

        // Third row (line 4): blank slug stays None.
        let (line, r1) = &reqs[1];
        assert_eq!(*line, 4);
        assert_eq!(r1.url, "https://b.com");
        assert_eq!(r1.slug, None);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bool_and_field_mapping() {
        assert_eq!(parse_bool("YES"), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("maybe"), None);

        let mut req = CreateLinkRequest::default();
        assert!(set_field(&mut req, "URL", "https://x.com"));
        assert!(set_field(&mut req, "cloaking", "on"));
        assert!(!set_field(&mut req, "nope", "v"));
        assert_eq!(req.url, "https://x.com");
        assert_eq!(req.cloaking, Some(true));
    }

    #[test]
    fn template_has_all_columns() {
        let path = tmp_csv("");
        write_template(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let header = text.lines().next().unwrap();
        assert!(header.starts_with("url,slug,name"));
        assert_eq!(header.split(',').count(), SUPPORTED.len());
        assert!(text.contains("https://example.com/landing"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn writes_result_csvs() {
        let dir = std::env::temp_dir().join(format!("linkly-res-{}", rand_name()));
        std::fs::create_dir_all(&dir).unwrap();

        let links = vec![NewLink {
            id: Some(7),
            slug: Some("/promo".to_string()),
            name: Some("Promo".to_string()),
            full_url: Some("https://go.me/promo".to_string()),
            url: Some("https://a.com".to_string()),
        }];
        let sp = write_success(&dir, &links).unwrap();
        let s = std::fs::read_to_string(&sp).unwrap();
        assert!(s.contains("id,slug,name,full_url,url"));
        assert!(s.contains("https://go.me/promo"));

        let fp = write_failures(&dir, &[(5, "boom".to_string())]).unwrap();
        let f = std::fs::read_to_string(&fp).unwrap();
        assert!(f.contains("line,reason"));
        assert!(f.contains("5,boom"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
