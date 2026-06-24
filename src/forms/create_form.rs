//! State and request-builder for the "create link" form.
//!
//! The form keeps one [`Input`] per text field plus a handful of boolean
//! toggles. [`CreateForm::build`] is a pure function from form state to a
//! [`CreateLinkRequest`]; the future CSV importer is intended to reuse the same
//! request type without touching this UI state.

use ratatui::widgets::ListState;
use tui_input::Input;

use crate::api::models::CreateLinkRequest;

/// Every focusable element of the form, in no particular order. The visible &
/// focusable subset and ordering is produced by [`CreateForm::fields`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    // Core
    Url,
    Name,
    Slug,
    Domain,
    Note,
    Enabled,
    UtmSource,
    UtmMedium,
    UtmCampaign,
    // Advanced
    UtmTerm,
    UtmContent,
    OgTitle,
    OgDescription,
    OgImage,
    FbPixel,
    TiktokPixel,
    Gtm,
    Ga4,
    HeadTags,
    BodyTags,
    LinkifyWords,
    ForwardParams,
    Cloaking,
    BlockBots,
    HideReferrer,
    // Action
    Submit,
}

/// Popup state for selecting a custom domain.
pub struct DomainSelector {
    /// `None` represents the workspace's default domain (no `domain` field sent).
    pub options: Vec<Option<String>>,
    pub state: ListState,
}

impl DomainSelector {
    pub fn new(domains: &[String]) -> Self {
        let mut options = vec![None];
        options.extend(domains.iter().cloned().map(Some));
        let mut state = ListState::default();
        state.select(Some(0));
        Self { options, state }
    }

    pub fn move_up(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(i.saturating_sub(1)));
    }

    pub fn move_down(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some((i + 1).min(self.options.len().saturating_sub(1))));
    }

    pub fn selected(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|i| self.options.get(i))
            .cloned()
            .flatten()
    }
}

#[derive(Default)]
pub struct CreateForm {
    pub url: Input,
    pub name: Input,
    pub slug: Input,
    pub note: Input,
    pub utm_source: Input,
    pub utm_medium: Input,
    pub utm_campaign: Input,
    pub utm_term: Input,
    pub utm_content: Input,
    pub og_title: Input,
    pub og_description: Input,
    pub og_image: Input,
    pub fb_pixel_id: Input,
    pub tiktok_pixel_id: Input,
    pub gtm_id: Input,
    pub ga4_tag_id: Input,
    pub head_tags: Input,
    pub body_tags: Input,
    pub linkify_words: Input,

    pub enabled: bool,
    pub forward_params: bool,
    pub cloaking: bool,
    pub block_bots: bool,
    pub hide_referrer: bool,

    /// Selected custom domain (`None` = default domain).
    pub domain: Option<String>,

    pub show_advanced: bool,
    pub focus: usize,
    pub domain_selector: Option<DomainSelector>,
}

impl CreateForm {
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// The ordered list of currently focusable fields.
    pub fn fields(&self) -> Vec<Field> {
        use Field::*;
        let mut v = vec![
            Url, Name, Slug, Domain, Note, Enabled, UtmSource, UtmMedium, UtmCampaign,
        ];
        if self.show_advanced {
            v.extend([
                UtmTerm,
                UtmContent,
                OgTitle,
                OgDescription,
                OgImage,
                FbPixel,
                TiktokPixel,
                Gtm,
                Ga4,
                HeadTags,
                BodyTags,
                LinkifyWords,
                ForwardParams,
                Cloaking,
                BlockBots,
                HideReferrer,
            ]);
        }
        v.push(Submit);
        v
    }

    pub fn current(&self) -> Field {
        let fields = self.fields();
        fields[self.focus.min(fields.len() - 1)]
    }

    pub fn focus_next(&mut self) {
        let len = self.fields().len();
        self.focus = (self.focus + 1) % len;
    }

    pub fn focus_prev(&mut self) {
        let len = self.fields().len();
        self.focus = (self.focus + len - 1) % len;
    }

    pub fn toggle_advanced(&mut self) {
        self.show_advanced = !self.show_advanced;
        // Keep focus in range when advanced fields disappear.
        let len = self.fields().len();
        if self.focus >= len {
            self.focus = len - 1;
        }
    }

    /// Mutable text input for a field, if it is a text field.
    pub fn input_mut(&mut self, f: Field) -> Option<&mut Input> {
        use Field::*;
        Some(match f {
            Url => &mut self.url,
            Name => &mut self.name,
            Slug => &mut self.slug,
            Note => &mut self.note,
            UtmSource => &mut self.utm_source,
            UtmMedium => &mut self.utm_medium,
            UtmCampaign => &mut self.utm_campaign,
            UtmTerm => &mut self.utm_term,
            UtmContent => &mut self.utm_content,
            OgTitle => &mut self.og_title,
            OgDescription => &mut self.og_description,
            OgImage => &mut self.og_image,
            FbPixel => &mut self.fb_pixel_id,
            TiktokPixel => &mut self.tiktok_pixel_id,
            Gtm => &mut self.gtm_id,
            Ga4 => &mut self.ga4_tag_id,
            HeadTags => &mut self.head_tags,
            BodyTags => &mut self.body_tags,
            LinkifyWords => &mut self.linkify_words,
            _ => return None,
        })
    }

    pub fn input(&self, f: Field) -> Option<&Input> {
        use Field::*;
        Some(match f {
            Url => &self.url,
            Name => &self.name,
            Slug => &self.slug,
            Note => &self.note,
            UtmSource => &self.utm_source,
            UtmMedium => &self.utm_medium,
            UtmCampaign => &self.utm_campaign,
            UtmTerm => &self.utm_term,
            UtmContent => &self.utm_content,
            OgTitle => &self.og_title,
            OgDescription => &self.og_description,
            OgImage => &self.og_image,
            FbPixel => &self.fb_pixel_id,
            TiktokPixel => &self.tiktok_pixel_id,
            Gtm => &self.gtm_id,
            Ga4 => &self.ga4_tag_id,
            HeadTags => &self.head_tags,
            BodyTags => &self.body_tags,
            LinkifyWords => &self.linkify_words,
            _ => return None,
        })
    }

    /// Read/toggle boolean fields. Returns `None` for non-boolean fields.
    pub fn bool_value(&self, f: Field) -> Option<bool> {
        use Field::*;
        Some(match f {
            Enabled => self.enabled,
            ForwardParams => self.forward_params,
            Cloaking => self.cloaking,
            BlockBots => self.block_bots,
            HideReferrer => self.hide_referrer,
            _ => return None,
        })
    }

    pub fn toggle_bool(&mut self, f: Field) {
        use Field::*;
        match f {
            Enabled => self.enabled = !self.enabled,
            ForwardParams => self.forward_params = !self.forward_params,
            Cloaking => self.cloaking = !self.cloaking,
            BlockBots => self.block_bots = !self.block_bots,
            HideReferrer => self.hide_referrer = !self.hide_referrer,
            _ => {}
        }
    }

    pub fn label(f: Field) -> &'static str {
        use Field::*;
        match f {
            Url => "URL *",
            Name => "Name",
            Slug => "Slug",
            Domain => "Domain",
            Note => "Note",
            Enabled => "Enabled",
            UtmSource => "UTM source",
            UtmMedium => "UTM medium",
            UtmCampaign => "UTM campaign",
            UtmTerm => "UTM term",
            UtmContent => "UTM content",
            OgTitle => "OG title",
            OgDescription => "OG description",
            OgImage => "OG image",
            FbPixel => "Facebook pixel",
            TiktokPixel => "TikTok pixel",
            Gtm => "GTM id",
            Ga4 => "GA4 tag id",
            HeadTags => "Head tags",
            BodyTags => "Body tags",
            LinkifyWords => "Linkify words",
            ForwardParams => "Forward params",
            Cloaking => "Cloaking",
            BlockBots => "Block bots",
            HideReferrer => "Hide referrer",
            Submit => "Submit",
        }
    }

    /// Display value for the domain field.
    pub fn domain_display(&self) -> String {
        self.domain
            .clone()
            .unwrap_or_else(|| "(default domain)".to_string())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.url.value().trim().is_empty() {
            return Err("URL is required".to_string());
        }
        Ok(())
    }

    /// Build the API request body from the current form state.
    pub fn build(&self, workspace_id: i64) -> CreateLinkRequest {
        fn opt(input: &Input) -> Option<String> {
            let v = input.value().trim();
            if v.is_empty() {
                None
            } else {
                Some(v.to_string())
            }
        }

        CreateLinkRequest {
            workspace_id,
            url: self.url.value().trim().to_string(),
            domain: self.domain.clone(),
            slug: opt(&self.slug),
            name: opt(&self.name),
            note: opt(&self.note),
            enabled: Some(self.enabled),
            forward_params: self.forward_params.then_some(true),
            cloaking: self.cloaking.then_some(true),
            block_bots: self.block_bots.then_some(true),
            hide_referrer: self.hide_referrer.then_some(true),
            og_title: opt(&self.og_title),
            og_description: opt(&self.og_description),
            og_image: opt(&self.og_image),
            utm_source: opt(&self.utm_source),
            utm_medium: opt(&self.utm_medium),
            utm_campaign: opt(&self.utm_campaign),
            utm_term: opt(&self.utm_term),
            utm_content: opt(&self.utm_content),
            fb_pixel_id: opt(&self.fb_pixel_id),
            tiktok_pixel_id: opt(&self.tiktok_pixel_id),
            gtm_id: opt(&self.gtm_id),
            ga4_tag_id: opt(&self.ga4_tag_id),
            head_tags: opt(&self.head_tags),
            body_tags: opt(&self.body_tags),
            linkify_words: opt(&self.linkify_words),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_form_serializes_only_required_and_enabled() {
        let mut form = CreateForm::new();
        form.url = Input::new("https://example.com".to_string());
        let body = form.build(42);
        let json = serde_json::to_value(&body).unwrap();
        let obj = json.as_object().unwrap();

        assert_eq!(obj["url"], "https://example.com");
        assert_eq!(obj["workspace_id"], 42);
        assert_eq!(obj["enabled"], true); // default enabled
        // Empty optional fields are omitted entirely.
        assert!(!obj.contains_key("slug"));
        assert!(!obj.contains_key("domain"));
        assert!(!obj.contains_key("utm_source"));
        // Off-by-default booleans are omitted, not sent as false.
        assert!(!obj.contains_key("cloaking"));
        assert!(!obj.contains_key("block_bots"));
    }

    #[test]
    fn populated_fields_and_domain_are_sent() {
        let mut form = CreateForm::new();
        form.url = Input::new("https://example.com".to_string());
        form.slug = Input::new("  promo  ".to_string()); // trimmed
        form.utm_source = Input::new("newsletter".to_string());
        form.cloaking = true;
        form.domain = Some("go.example.com".to_string());

        let json = serde_json::to_value(form.build(7)).unwrap();
        let obj = json.as_object().unwrap();

        assert_eq!(obj["slug"], "promo");
        assert_eq!(obj["utm_source"], "newsletter");
        assert_eq!(obj["cloaking"], true);
        assert_eq!(obj["domain"], "go.example.com");
    }

    #[test]
    fn validate_requires_url() {
        let form = CreateForm::new();
        assert!(form.validate().is_err());
    }

    #[test]
    fn advanced_toggle_changes_focusable_fields() {
        let mut form = CreateForm::new();
        let core = form.fields().len();
        form.toggle_advanced();
        assert!(form.fields().len() > core);
    }
}
