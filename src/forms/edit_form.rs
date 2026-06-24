//! State for viewing and editing an existing link in the detail screen.
//!
//! A [`LinkEditor`] is built from the link record returned by the API. It holds
//! one entry per editable field with both the live value and the value as last
//! saved, so "dirty" state is simply "anything differs from its saved value".
//! After a successful save, [`LinkEditor::mark_saved`] snapshots the live values
//! as the new saved baseline — so the saved state *is* the TUI state and we stop
//! prompting to save.

use serde_json::Value;
use tui_input::Input;

/// The fields a user can edit. These mirror the writable fields of the create
/// endpoint; read-only fields (id, clicks, full_url, …) are shown in the title.
const EDITABLE: &[(&str, &str, EditKind)] = &[
    ("url", "URL", EditKind::Text),
    ("name", "Name", EditKind::Text),
    ("slug", "Slug", EditKind::Text),
    ("domain", "Domain", EditKind::Text),
    ("note", "Note", EditKind::Text),
    ("enabled", "Enabled", EditKind::Bool),
    ("forward_params", "Forward params", EditKind::Bool),
    ("cloaking", "Cloaking", EditKind::Bool),
    ("block_bots", "Block bots", EditKind::Bool),
    ("hide_referrer", "Hide referrer", EditKind::Bool),
    ("og_title", "OG title", EditKind::Text),
    ("og_description", "OG description", EditKind::Text),
    ("og_image", "OG image", EditKind::Text),
    ("utm_source", "UTM source", EditKind::Text),
    ("utm_medium", "UTM medium", EditKind::Text),
    ("utm_campaign", "UTM campaign", EditKind::Text),
    ("utm_term", "UTM term", EditKind::Text),
    ("utm_content", "UTM content", EditKind::Text),
    ("fb_pixel_id", "Facebook pixel", EditKind::Text),
    ("tiktok_pixel_id", "TikTok pixel", EditKind::Text),
    ("gtm_id", "GTM id", EditKind::Text),
    ("ga4_tag_id", "GA4 tag id", EditKind::Text),
    ("head_tags", "Head tags", EditKind::Text),
    ("body_tags", "Body tags", EditKind::Text),
    ("linkify_words", "Linkify words", EditKind::Text),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Text,
    Bool,
}

/// Sub-mode of the detail screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailMode {
    /// Moving between fields.
    Nav,
    /// Editing the current text field.
    Edit,
    /// The "save before leaving?" prompt.
    ConfirmSave,
}

pub struct EditField {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: EditKind,
    pub input: Input,
    pub bool_val: bool,
    saved_text: String,
    saved_bool: bool,
}

impl EditField {
    pub fn changed(&self) -> bool {
        match self.kind {
            EditKind::Text => self.input.value() != self.saved_text,
            EditKind::Bool => self.bool_val != self.saved_bool,
        }
    }
}

pub struct LinkEditor {
    pub id: i64,
    pub workspace_id: i64,
    pub full_url: String,
    pub fields: Vec<EditField>,
    pub cursor: usize,
    pub mode: DetailMode,
    /// Set when the user triggered a save while trying to leave, so we exit to
    /// the list once the save succeeds.
    pub exit_after_save: bool,
}

impl LinkEditor {
    pub fn from_value(v: &Value, fallback_ws: i64) -> Self {
        let id = v.get("id").and_then(Value::as_i64).unwrap_or(0);
        let workspace_id = v
            .get("workspace_id")
            .and_then(Value::as_i64)
            .unwrap_or(fallback_ws);
        let full_url = v
            .get("full_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let fields = EDITABLE
            .iter()
            .map(|&(key, label, kind)| match kind {
                EditKind::Text => {
                    let s = v
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    EditField {
                        key,
                        label,
                        kind,
                        input: Input::new(s.clone()),
                        bool_val: false,
                        saved_text: s,
                        saved_bool: false,
                    }
                }
                EditKind::Bool => {
                    let b = v.get(key).and_then(Value::as_bool).unwrap_or(false);
                    EditField {
                        key,
                        label,
                        kind,
                        input: Input::default(),
                        bool_val: b,
                        saved_text: String::new(),
                        saved_bool: b,
                    }
                }
            })
            .collect();

        Self {
            id,
            workspace_id,
            full_url,
            fields,
            cursor: 0,
            mode: DetailMode::Nav,
            exit_after_save: false,
        }
    }

    pub fn dirty(&self) -> bool {
        self.fields.iter().any(EditField::changed)
    }

    pub fn current(&self) -> &EditField {
        &self.fields[self.cursor]
    }

    pub fn current_mut(&mut self) -> &mut EditField {
        &mut self.fields[self.cursor]
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.cursor = (self.cursor + 1).min(self.fields.len() - 1);
    }

    /// The update payload: `id` + `workspace_id` plus only the changed fields,
    /// so unchanged fields are left untouched server-side.
    pub fn update_body(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("id".to_string(), self.id.into());
        map.insert("workspace_id".to_string(), self.workspace_id.into());
        for f in &self.fields {
            if f.changed() {
                let value = match f.kind {
                    EditKind::Text => Value::String(f.input.value().to_string()),
                    EditKind::Bool => Value::Bool(f.bool_val),
                };
                map.insert(f.key.to_string(), value);
            }
        }
        Value::Object(map)
    }

    /// Adopt the live values as the new saved baseline (after a successful save).
    pub fn mark_saved(&mut self) {
        for f in &mut self.fields {
            f.saved_text = f.input.value().to_string();
            f.saved_bool = f.bool_val;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> LinkEditor {
        let v = json!({
            "id": 30,
            "workspace_id": 42,
            "full_url": "https://go.example.com/promo",
            "name": "Promo",
            "url": "https://example.com",
            "enabled": true,
        });
        LinkEditor::from_value(&v, 0)
    }

    fn field_mut<'a>(e: &'a mut LinkEditor, key: &str) -> &'a mut EditField {
        e.fields.iter_mut().find(|f| f.key == key).unwrap()
    }

    #[test]
    fn freshly_loaded_editor_is_not_dirty() {
        let e = sample();
        assert_eq!(e.id, 30);
        assert_eq!(e.workspace_id, 42);
        assert!(!e.dirty());
    }

    #[test]
    fn editing_a_field_makes_it_dirty_and_appears_in_update() {
        let mut e = sample();
        field_mut(&mut e, "name").input = Input::new("Renamed".to_string());
        assert!(e.dirty());

        let body = e.update_body();
        let obj = body.as_object().unwrap();
        assert_eq!(obj["id"], 30);
        assert_eq!(obj["workspace_id"], 42);
        assert_eq!(obj["name"], "Renamed");
        // Untouched fields are not sent.
        assert!(!obj.contains_key("url"));
    }

    #[test]
    fn toggling_a_bool_is_tracked() {
        let mut e = sample();
        field_mut(&mut e, "enabled").bool_val = false;
        let body = e.update_body();
        assert_eq!(body.as_object().unwrap()["enabled"], false);
    }

    #[test]
    fn mark_saved_clears_dirty() {
        let mut e = sample();
        field_mut(&mut e, "name").input = Input::new("Renamed".to_string());
        assert!(e.dirty());
        e.mark_saved();
        assert!(!e.dirty());
        // After saving, an unchanged update body carries only id + workspace_id.
        let obj = e.update_body();
        assert_eq!(obj.as_object().unwrap().len(), 2);
    }
}
