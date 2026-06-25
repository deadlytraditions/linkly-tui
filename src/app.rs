//! Application state machine, event dispatch, and async orchestration.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::api::models::{CreateLinkRequest, LinkSummary, ListLinksResponse, Workspace};
use crate::api::LinklyClient;
use crate::config::CachedWorkspace;
use crate::forms::import::{ImportStage, NewLink, Progress, Summary};
use crate::forms::{CreateForm, DetailMode, DomainSelector, EditKind, Field, ImportState, LinkEditor};

pub const PAGE_SIZE: i64 = 100;

/// Which screen is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    WorkspacePicker,
    Auth,
    LinkList,
    LinkDetail,
    CreateLink,
    Import,
}

/// The columns the link list can be sorted by. Each maps to an API `sort_by`
/// field that matches a visible table column, so the chosen sort is obvious.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Created,
    Name,
    Slug,
    ClicksTotal,
    ClicksThirtyDays,
    ClicksToday,
}

impl SortField {
    pub const ALL: [SortField; 6] = [
        SortField::Created,
        SortField::Name,
        SortField::Slug,
        SortField::ClicksTotal,
        SortField::ClicksThirtyDays,
        SortField::ClicksToday,
    ];

    /// The API `sort_by` value.
    pub fn api_field(self) -> &'static str {
        match self {
            SortField::Created => "id",
            SortField::Name => "name",
            SortField::Slug => "slug",
            SortField::ClicksTotal => "clicks_total",
            SortField::ClicksThirtyDays => "clicks_thirty_days",
            SortField::ClicksToday => "clicks_today",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortField::Created => "Created",
            SortField::Name => "Name",
            SortField::Slug => "Slug",
            SortField::ClicksTotal => "Total clicks",
            SortField::ClicksThirtyDays => "Clicks (30d)",
            SortField::ClicksToday => "Clicks (today)",
        }
    }
}

/// Results delivered back to the UI from spawned API tasks.
pub enum AsyncMsg {
    LinksLoaded(ListLinksResponse),
    LinkDetailLoaded(Value),
    DomainsLoaded(Vec<String>),
    LinkCreated(NewLink),
    LinkUpdated,
    WorkspacesLoaded(Vec<Workspace>),
    QrExported { count: usize, dir: String },
    ImportProgress { done: usize, ok: usize, failed: usize },
    ImportDone {
        ok: usize,
        failed: usize,
        success_path: Option<String>,
        failure_path: Option<String>,
        new_links: Vec<NewLink>,
    },
    Error(String),
}

/// A link captured for single QR export (so the dialog can render the chosen
/// format before writing).
pub struct QrSingle {
    pub id: Option<i64>,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub url: String,
}

/// What the QR dialog should export when confirmed (`None` = just edit settings).
pub enum QrExportTarget {
    Single(QrSingle),
    Workspace,
}

/// The credential prompt shown on startup.
#[derive(Default)]
pub struct AuthState {
    pub api_key: Input,
    pub workspace_id: Input,
    /// 0 = api key, 1 = workspace id.
    pub focus: usize,
    pub error: Option<String>,
    /// When `true` the workspace was chosen from the cache, so only the API key
    /// is requested and `ws_name` is shown for context.
    pub ws_locked: bool,
    pub ws_name: String,
}

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
    pub status: String,
    pub loading: bool,

    pub auth: AuthState,
    pub client: Option<LinklyClient>,
    pub workspace_id: i64,

    // Workspace picker (startup).
    pub cached_workspaces: Vec<CachedWorkspace>,
    pub picker_cursor: usize,

    /// The API key used for the current session (for the optional store offer).
    current_key: String,
    /// Set when authenticating; the first successful link load clears it,
    /// caches the (now verified) workspace, and may offer to store the key.
    verify_pending: bool,
    /// Workspace name fetched from the API while verification is still pending,
    /// applied once the workspace is actually cached.
    pending_ws_name: Option<String>,
    /// Whether the "store this API key?" prompt is showing.
    pub store_prompt: bool,

    // QR export settings + editor popup.
    pub qr_settings: crate::qr::QrSettings,
    pub qr_settings_open: bool,
    pub qr_form_focus: usize,
    pub qr_size_input: Input,
    pub qr_fg_input: Input,
    pub qr_bg_input: Input,
    /// What to export once the QR dialog is confirmed (`None` = settings only).
    qr_pending: Option<QrExportTarget>,

    /// CSV import flow (file browser → preview → run → done), if active.
    pub import: Option<ImportState>,

    /// Whether the keybindings help overlay is showing.
    pub help_open: bool,

    // Link list state.
    pub links: Vec<LinkSummary>,
    pub list_state: TableState,
    pub page: i64,
    pub total_pages: i64,
    pub total_entries: i64,
    pub search: String,
    pub search_input: Input,
    pub searching: bool,

    // Sorting.
    pub sort_field: SortField,
    pub sort_desc: bool,
    /// Sort picker popup state.
    pub sort_open: bool,
    pub sort_cursor: usize,
    pub sort_cursor_desc: bool,

    // Detail / edit state.
    pub editor: Option<LinkEditor>,

    // Create state.
    pub create_form: CreateForm,
    pub domains: Vec<String>,

    tx: UnboundedSender<AsyncMsg>,
}

impl App {
    pub fn new(tx: UnboundedSender<AsyncMsg>) -> Self {
        let (key, ws) = crate::config::env_prefill();
        let auth = AuthState {
            api_key: Input::new(key),
            workspace_id: Input::new(ws),
            focus: 0,
            error: None,
            ws_locked: false,
            ws_name: String::new(),
        };
        let cached_workspaces = crate::config::load_workspaces();
        // Show the picker first if we have remembered workspaces; otherwise go
        // straight to the full sign-in form.
        let screen = if cached_workspaces.is_empty() {
            Screen::Auth
        } else {
            Screen::WorkspacePicker
        };
        Self {
            screen,
            should_quit: false,
            status: String::new(),
            loading: false,
            auth,
            client: None,
            workspace_id: 0,
            cached_workspaces,
            picker_cursor: 0,
            current_key: String::new(),
            verify_pending: false,
            pending_ws_name: None,
            store_prompt: false,
            qr_settings: crate::config::load_qr_settings(),
            qr_settings_open: false,
            qr_form_focus: 0,
            qr_size_input: Input::default(),
            qr_fg_input: Input::default(),
            qr_bg_input: Input::default(),
            qr_pending: None,
            import: None,
            help_open: false,
            links: Vec::new(),
            list_state: TableState::default(),
            page: 1,
            total_pages: 1,
            total_entries: 0,
            search: String::new(),
            search_input: Input::default(),
            searching: false,
            sort_field: SortField::Created,
            sort_desc: true,
            sort_open: false,
            sort_cursor: 0,
            sort_cursor_desc: true,
            editor: None,
            create_form: CreateForm::new(),
            domains: Vec::new(),
            tx,
        }
    }

    // ------------------------------------------------------------------
    // Event handling
    // ------------------------------------------------------------------

    pub fn on_event(&mut self, event: Event) {
        let Event::Key(key) = event else { return };
        // The QR dialog is a global overlay; handle it on any screen.
        if self.qr_settings_open {
            self.on_qr_settings_key(key);
            return;
        }
        // The help overlay swallows keys; dismiss on the obvious ones.
        if self.help_open {
            if matches!(
                key.code,
                KeyCode::Esc
                    | KeyCode::Enter
                    | KeyCode::Char('?')
                    | KeyCode::Char('q')
                    | KeyCode::Char('h')
            ) {
                self.help_open = false;
            }
            return;
        }
        // `?` opens help on any screen, except where a text field is focused
        // (so URLs/queries containing `?` can still be typed).
        if key.code == KeyCode::Char('?') && self.question_opens_help() {
            self.help_open = true;
            return;
        }
        match self.screen {
            Screen::WorkspacePicker => self.on_picker_key(key),
            Screen::Auth => self.on_auth_key(key),
            Screen::LinkList => self.on_list_key(key),
            Screen::LinkDetail => self.on_detail_key(key),
            Screen::CreateLink => self.on_create_key(key),
            Screen::Import => self.on_import_key(key),
        }
    }

    /// The number of selectable rows in the picker: cached workspaces + the
    /// trailing "add new" entry.
    fn picker_len(&self) -> usize {
        self.cached_workspaces.len() + 1
    }

    /// True when the picker cursor is on the "+ Add new workspace" row.
    fn picker_on_add(&self) -> bool {
        self.picker_cursor >= self.cached_workspaces.len()
    }

    fn on_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.picker_cursor = self.picker_cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.picker_cursor = (self.picker_cursor + 1).min(self.picker_len() - 1);
            }
            KeyCode::Char('d') if !self.picker_on_add() => {
                self.cached_workspaces.remove(self.picker_cursor);
                crate::config::save_workspaces(&self.cached_workspaces);
                self.picker_cursor = self
                    .picker_cursor
                    .min(self.cached_workspaces.len().saturating_sub(1));
                if self.cached_workspaces.is_empty() {
                    self.picker_cursor = 0;
                }
            }
            KeyCode::Enter => {
                if self.picker_on_add() {
                    self.start_auth(None);
                } else {
                    let ws = self.cached_workspaces[self.picker_cursor].clone();
                    self.start_auth(Some(ws));
                }
            }
            _ => {}
        }
    }

    /// Whether pressing `?` should open the help overlay on the current screen.
    /// Returns false when a text field is focused so `?` types normally.
    fn question_opens_help(&self) -> bool {
        match self.screen {
            Screen::Auth => false,
            Screen::WorkspacePicker => true,
            Screen::Import => true, // no text fields in any import stage
            Screen::LinkList => !self.searching && !self.sort_open && !self.store_prompt,
            Screen::LinkDetail => self
                .editor
                .as_ref()
                .map(|e| e.mode == DetailMode::Nav)
                .unwrap_or(true),
            Screen::CreateLink => {
                self.create_form.domain_selector.is_none()
                    && self.create_form.input(self.create_form.current()).is_none()
            }
        }
    }

    /// Move to the API-key prompt. `Some(ws)` locks to a cached workspace (key
    /// only); `None` asks for both the key and a workspace id.
    fn start_auth(&mut self, ws: Option<CachedWorkspace>) {
        let (env_key, env_ws) = crate::config::env_prefill();
        self.auth.error = None;
        self.auth.focus = 0;
        match ws {
            Some(ws) => {
                self.workspace_id = ws.id;
                self.auth.ws_locked = true;
                self.auth.ws_name = ws.name;
                self.auth.workspace_id = Input::new(ws.id.to_string());
                // Pre-fill a stored key (if any) so the user can just press Enter.
                self.auth.api_key = Input::new(ws.api_key.unwrap_or(env_key));
            }
            None => {
                self.auth.ws_locked = false;
                self.auth.ws_name = String::new();
                self.auth.workspace_id = Input::new(env_ws);
                self.auth.api_key = Input::new(env_key);
            }
        }
        self.screen = Screen::Auth;
    }

    fn on_auth_key(&mut self, key: KeyEvent) {
        // Number of focusable fields: 1 when the workspace is locked, else 2.
        let fields = if self.auth.ws_locked { 1 } else { 2 };
        match key.code {
            KeyCode::Esc => {
                // Back to the picker if we have one, otherwise quit.
                if self.cached_workspaces.is_empty() {
                    self.should_quit = true;
                } else {
                    self.screen = Screen::WorkspacePicker;
                }
            }
            KeyCode::Tab | KeyCode::Down => self.auth.focus = (self.auth.focus + 1) % fields,
            KeyCode::Up | KeyCode::BackTab => {
                self.auth.focus = (self.auth.focus + fields - 1) % fields;
            }
            KeyCode::Enter => self.try_authenticate(),
            _ => {
                let field = if self.auth.focus == 0 {
                    &mut self.auth.api_key
                } else {
                    &mut self.auth.workspace_id
                };
                field.handle_event(&Event::Key(key));
            }
        }
    }

    fn try_authenticate(&mut self) {
        let key = self.auth.api_key.value().trim().to_string();
        if key.is_empty() {
            self.auth.error = Some("API key is required".to_string());
            return;
        }
        // A locked (cached) workspace already has its id; otherwise parse input.
        let ws_id = if self.auth.ws_locked {
            self.workspace_id
        } else {
            let ws = self.auth.workspace_id.value().trim().to_string();
            match ws.parse::<i64>() {
                Ok(id) => id,
                Err(_) => {
                    self.auth.error = Some("Workspace ID must be a number".to_string());
                    return;
                }
            }
        };
        self.auth.error = None;
        self.client = Some(LinklyClient::new(key.clone()));
        self.workspace_id = ws_id;
        self.current_key = key;
        self.pending_ws_name = None;
        // The workspace is cached only after the first successful link load
        // confirms the key and workspace id actually work (see `LinksLoaded`).
        self.verify_pending = true;
        self.screen = Screen::LinkList;
        self.page = 1;
        self.status = "Loading links…".to_string();
        self.loading = true;
        self.load_links(1);
        self.load_domains();
        self.load_workspaces();
    }

    fn on_list_key(&mut self, key: KeyEvent) {
        // The "store API key?" prompt takes precedence over everything else.
        if self.store_prompt {
            match key.code {
                KeyCode::Char('s') | KeyCode::Char('y') => {
                    self.store_active_key();
                    self.store_prompt = false;
                    self.status = "API key stored for this workspace".to_string();
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.store_prompt = false;
                    self.status = "API key not stored".to_string();
                }
                _ => {}
            }
            return;
        }

        // Sort picker popup takes precedence.
        if self.sort_open {
            match key.code {
                KeyCode::Esc => self.sort_open = false,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.sort_cursor = self.sort_cursor.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.sort_cursor = (self.sort_cursor + 1).min(SortField::ALL.len() - 1);
                }
                KeyCode::Left
                | KeyCode::Right
                | KeyCode::Char('h')
                | KeyCode::Char('l')
                | KeyCode::Char('d')
                | KeyCode::Tab => {
                    self.sort_cursor_desc = !self.sort_cursor_desc;
                }
                KeyCode::Enter => {
                    self.sort_field = SortField::ALL[self.sort_cursor];
                    self.sort_desc = self.sort_cursor_desc;
                    self.sort_open = false;
                    self.page = 1;
                    self.reload("Sorting…", 1);
                }
                _ => {}
            }
            return;
        }

        if self.searching {
            match key.code {
                KeyCode::Enter => {
                    self.search = self.search_input.value().trim().to_string();
                    self.searching = false;
                    self.page = 1;
                    self.reload("Searching…", 1);
                }
                KeyCode::Esc => {
                    self.searching = false;
                    self.search_input = Input::default();
                }
                _ => {
                    self.search_input.handle_event(&Event::Key(key));
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            // Esc steps back to the workspace picker (Esc again there quits).
            KeyCode::Esc => {
                self.screen = Screen::WorkspacePicker;
                self.picker_cursor = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_prev(),
            KeyCode::Enter => self.open_detail(),
            KeyCode::Char('c') => self.open_create(),
            KeyCode::Char('Q') => self.open_qr_settings(Some(QrExportTarget::Workspace)),
            KeyCode::Char('o') => self.open_qr_settings(None),
            KeyCode::Char('i') => self.open_import(),
            KeyCode::Char('r') => self.reload("Refreshing…", self.page),
            KeyCode::Char('/') => {
                self.searching = true;
                self.search_input = Input::new(self.search.clone());
            }
            KeyCode::Char('s') => {
                self.sort_open = true;
                self.sort_cursor = SortField::ALL
                    .iter()
                    .position(|f| *f == self.sort_field)
                    .unwrap_or(0);
                self.sort_cursor_desc = self.sort_desc;
            }
            KeyCode::Char('n') => {
                // Request the next page; the counter only moves once it loads.
                if self.page < self.total_pages {
                    self.reload("Loading…", self.page + 1);
                }
            }
            KeyCode::Char('p') => {
                if self.page > 1 {
                    self.reload("Loading…", self.page - 1);
                }
            }
            _ => {}
        }
    }

    fn on_detail_key(&mut self, key: KeyEvent) {
        // If the record hasn't loaded yet, only allow backing out.
        let Some(mode) = self.editor.as_ref().map(|e| e.mode) else {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.exit_detail();
            }
            return;
        };
        match mode {
            DetailMode::Nav => self.detail_nav_key(key),
            DetailMode::Edit => self.detail_edit_key(key),
            DetailMode::ConfirmSave => self.detail_confirm_key(key),
        }
    }

    fn detail_nav_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(e) = self.editor.as_mut() {
                    e.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(e) = self.editor.as_mut() {
                    e.move_down();
                }
            }
            KeyCode::Enter => self.detail_enter(),
            KeyCode::Char('Q') => self.begin_export_current_qr(),
            KeyCode::Char('s') => {
                if let Some(e) = self.editor.as_mut() {
                    e.exit_after_save = false;
                }
                self.save_edit();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                let dirty = self.editor.as_ref().is_some_and(LinkEditor::dirty);
                if dirty {
                    if let Some(e) = self.editor.as_mut() {
                        e.mode = DetailMode::ConfirmSave;
                    }
                } else {
                    self.exit_detail();
                }
            }
            _ => {}
        }
    }

    fn detail_enter(&mut self) {
        let Some(e) = self.editor.as_mut() else { return };
        match e.current().kind {
            EditKind::Bool => {
                let f = e.current_mut();
                f.bool_val = !f.bool_val;
            }
            EditKind::Text => e.mode = DetailMode::Edit,
        }
    }

    fn detail_edit_key(&mut self, key: KeyEvent) {
        let Some(e) = self.editor.as_mut() else { return };
        match key.code {
            // Enter and Esc both commit the in-progress edit (the value is
            // already live in the input) and return to navigation.
            KeyCode::Enter | KeyCode::Esc => e.mode = DetailMode::Nav,
            _ => {
                e.current_mut().input.handle_event(&Event::Key(key));
            }
        }
    }

    fn detail_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('y') => {
                if let Some(e) = self.editor.as_mut() {
                    e.exit_after_save = true;
                }
                self.save_edit();
            }
            KeyCode::Char('d') => self.exit_detail(), // discard changes
            KeyCode::Esc | KeyCode::Char('n') => {
                if let Some(e) = self.editor.as_mut() {
                    e.mode = DetailMode::Nav;
                }
            }
            _ => {}
        }
    }

    fn save_edit(&mut self) {
        let Some(e) = self.editor.as_ref() else { return };
        if !e.dirty() {
            if e.exit_after_save {
                self.exit_detail();
            }
            return;
        }
        let Some(client) = self.client.clone() else { return };
        let body = e.update_body();
        let tx = self.tx.clone();
        if let Some(e) = self.editor.as_mut() {
            e.mode = DetailMode::Nav;
        }
        self.status = "Saving link…".to_string();
        self.loading = true;
        tokio::spawn(async move {
            let msg = match client.update_link(body).await {
                Ok(_) => AsyncMsg::LinkUpdated,
                Err(e) => AsyncMsg::Error(e.to_string()),
            };
            let _ = tx.send(msg);
        });
    }

    /// Leave the detail screen and refresh the list (so edits are reflected,
    /// and the status line no longer shows detail-screen text).
    fn exit_detail(&mut self) {
        self.screen = Screen::LinkList;
        self.editor = None;
        self.reload("Refreshing…", self.page);
    }

    fn on_create_key(&mut self, key: KeyEvent) {
        // Domain selector popup takes precedence.
        if let Some(selector) = self.create_form.domain_selector.as_mut() {
            match key.code {
                KeyCode::Esc => self.create_form.domain_selector = None,
                KeyCode::Up | KeyCode::Char('k') => selector.move_up(),
                KeyCode::Down | KeyCode::Char('j') => selector.move_down(),
                KeyCode::Enter => {
                    self.create_form.domain = selector.selected();
                    self.create_form.domain_selector = None;
                }
                _ => {}
            }
            return;
        }

        // Ctrl-A toggles advanced fields regardless of focus.
        if key.code == KeyCode::Char('a') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.create_form.toggle_advanced();
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::LinkList;
            }
            KeyCode::Tab | KeyCode::Down => self.create_form.focus_next(),
            KeyCode::BackTab | KeyCode::Up => self.create_form.focus_prev(),
            KeyCode::Enter => self.on_create_enter(),
            KeyCode::Char(' ')
                if self.create_form.bool_value(self.create_form.current()).is_some() =>
            {
                let f = self.create_form.current();
                self.create_form.toggle_bool(f);
            }
            _ => {
                let f = self.create_form.current();
                if let Some(input) = self.create_form.input_mut(f) {
                    input.handle_event(&Event::Key(key));
                }
            }
        }
    }

    fn on_create_enter(&mut self) {
        match self.create_form.current() {
            Field::Domain => {
                self.create_form.domain_selector = Some(DomainSelector::new(&self.domains));
            }
            Field::Submit => self.submit_create(),
            f if self.create_form.bool_value(f).is_some() => self.create_form.toggle_bool(f),
            _ => self.create_form.focus_next(),
        }
    }

    fn submit_create(&mut self) {
        if let Err(e) = self.create_form.validate() {
            self.status = format!("Cannot submit: {e}");
            return;
        }
        let Some(client) = self.client.clone() else { return };
        let body = self.create_form.build(self.workspace_id);
        let tx = self.tx.clone();
        self.status = "Creating link…".to_string();
        self.loading = true;
        tokio::spawn(async move {
            let msg = match client.create_link(&body).await {
                Ok(v) => AsyncMsg::LinkCreated(NewLink::from_response(&v)),
                Err(e) => AsyncMsg::Error(e.to_string()),
            };
            let _ = tx.send(msg);
        });
    }

    // ------------------------------------------------------------------
    // List navigation helpers
    // ------------------------------------------------------------------

    fn select_next(&mut self) {
        if self.links.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(self.links.len() - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.links.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0).saturating_sub(1);
        self.list_state.select(Some(i));
    }

    fn open_detail(&mut self) {
        let Some(link) = self.selected_link() else { return };
        let Some(id) = link.id else { return };
        let Some(client) = self.client.clone() else { return };
        let ws = self.workspace_id;
        let tx = self.tx.clone();
        self.screen = Screen::LinkDetail;
        self.editor = None;
        self.status = "Loading link details…".to_string();
        self.loading = true;
        tokio::spawn(async move {
            let msg = match client.get_link(id, ws).await {
                Ok(v) => AsyncMsg::LinkDetailLoaded(v),
                Err(e) => AsyncMsg::Error(e.to_string()),
            };
            let _ = tx.send(msg);
        });
    }

    fn open_create(&mut self) {
        self.create_form = CreateForm::new();
        self.screen = Screen::CreateLink;
        self.status = "New link — fill in the URL and press Enter on Submit".to_string();
        if self.domains.is_empty() {
            self.load_domains();
        }
    }

    pub fn selected_link(&self) -> Option<&LinkSummary> {
        self.list_state.selected().and_then(|i| self.links.get(i))
    }

    fn reload(&mut self, status: &str, page: i64) {
        self.status = status.to_string();
        self.loading = true;
        self.load_links(page);
    }

    // ------------------------------------------------------------------
    // Async task spawners
    // ------------------------------------------------------------------

    /// Fetch `page`. The displayed `self.page` is only updated once a response
    /// actually arrives (see `LinksLoaded`), so a failed request never leaves
    /// the page counter out of sync with what's on screen.
    fn load_links(&self, page: i64) {
        let Some(client) = self.client.clone() else { return };
        let tx = self.tx.clone();
        let ws = self.workspace_id;
        let search = self.search.clone();
        let sort_by = self.sort_field.api_field();
        let sort_dir = if self.sort_desc { "desc" } else { "asc" };
        tokio::spawn(async move {
            let msg = match client
                .list_links(ws, page, PAGE_SIZE, &search, sort_by, sort_dir)
                .await
            {
                Ok(r) => AsyncMsg::LinksLoaded(r),
                Err(e) => AsyncMsg::Error(e.to_string()),
            };
            let _ = tx.send(msg);
        });
    }

    fn load_domains(&self) {
        let Some(client) = self.client.clone() else { return };
        let tx = self.tx.clone();
        let ws = self.workspace_id;
        tokio::spawn(async move {
            if let Ok(domains) = client.list_domains(ws).await {
                let _ = tx.send(AsyncMsg::DomainsLoaded(domains));
            }
        });
    }

    /// Open the QR dialog to export the currently selected/open link.
    fn begin_export_current_qr(&mut self) {
        let Some(link) = self.selected_link() else { return };
        let Some(url) = link.full_url.clone().filter(|u| !u.is_empty()) else {
            self.status = "No short URL to encode for this link".to_string();
            return;
        };
        let target = QrExportTarget::Single(QrSingle {
            id: link.id,
            slug: link.slug.clone(),
            name: link.name.clone(),
            url,
        });
        self.open_qr_settings(Some(target));
    }

    /// Label for the pending export (used by the dialog), if any.
    pub fn qr_export_label(&self) -> Option<&'static str> {
        match self.qr_pending {
            Some(QrExportTarget::Single(_)) => Some("link"),
            Some(QrExportTarget::Workspace) => Some("workspace"),
            None => None,
        }
    }

    fn on_qr_settings_key(&mut self, key: KeyEvent) {
        let fields = 4;
        match key.code {
            KeyCode::Enter => self.commit_qr_settings(true),
            KeyCode::Esc => self.commit_qr_settings(false),
            KeyCode::Up | KeyCode::BackTab => {
                self.qr_form_focus = (self.qr_form_focus + fields - 1) % fields;
            }
            KeyCode::Down | KeyCode::Tab => {
                self.qr_form_focus = (self.qr_form_focus + 1) % fields;
            }
            KeyCode::Left | KeyCode::Char('h') if self.qr_form_focus == 0 => {
                self.qr_settings.format = self.qr_settings.format.prev();
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') if self.qr_form_focus == 0 => {
                self.qr_settings.format = self.qr_settings.format.next();
            }
            _ => match self.qr_form_focus {
                1 => {
                    self.qr_size_input.handle_event(&Event::Key(key));
                }
                2 => {
                    self.qr_fg_input.handle_event(&Event::Key(key));
                }
                3 => {
                    self.qr_bg_input.handle_event(&Event::Key(key));
                }
                _ => {}
            },
        }
    }

    /// Write a single QR for a captured link using the current settings.
    fn export_single_qr(&mut self, s: &QrSingle) {
        let fname = crate::qr::file_name(
            s.id,
            s.slug.as_deref(),
            s.name.as_deref(),
            self.qr_settings.format,
        );
        let path = crate::qr::output_dir(self.workspace_id).join(fname);
        match crate::qr::write_qr(&s.url, &path, &self.qr_settings) {
            Ok(()) => self.status = format!("Saved QR to {}", path.display()),
            Err(e) => self.status = format!("Error: {e}"),
        }
    }

    /// Export QR codes for every link in the workspace (paging through all of
    /// them) on a background task.
    fn export_workspace_qr(&mut self) {
        let Some(client) = self.client.clone() else { return };
        let tx = self.tx.clone();
        let ws = self.workspace_id;
        let search = self.search.clone();
        let sort_by = self.sort_field.api_field().to_string();
        let sort_dir = if self.sort_desc { "desc" } else { "asc" }.to_string();
        let settings = self.qr_settings.clone();
        self.status = "Exporting QR codes…".to_string();
        self.loading = true;
        tokio::spawn(async move {
            let msg = export_all_qr(&client, ws, &search, &sort_by, &sort_dir, &settings).await;
            let _ = tx.send(msg);
        });
    }

    fn open_qr_settings(&mut self, pending: Option<QrExportTarget>) {
        self.qr_pending = pending;
        self.qr_form_focus = 0;
        self.qr_size_input = Input::new(self.qr_settings.size.to_string());
        self.qr_fg_input = Input::new(self.qr_settings.fg.clone());
        self.qr_bg_input = Input::new(self.qr_settings.bg.clone());
        self.qr_settings_open = true;
    }

    /// Close the QR dialog. When `confirmed`, persist the settings and run the
    /// pending export; when cancelled (Esc), do neither.
    fn commit_qr_settings(&mut self, confirmed: bool) {
        self.qr_settings_open = false;
        let pending = self.qr_pending.take();

        if !confirmed {
            self.status = "QR export cancelled".to_string();
            return;
        }

        if let Ok(sz) = self.qr_size_input.value().trim().parse::<u32>() {
            self.qr_settings.size = sz.clamp(64, 4096);
        }
        if let Some(c) = crate::qr::normalize_color(self.qr_fg_input.value()) {
            self.qr_settings.fg = c;
        }
        if let Some(c) = crate::qr::normalize_color(self.qr_bg_input.value()) {
            self.qr_settings.bg = c;
        }
        crate::config::save_qr_settings(&self.qr_settings);

        match pending {
            Some(QrExportTarget::Single(s)) => self.export_single_qr(&s),
            Some(QrExportTarget::Workspace) => self.export_workspace_qr(),
            None => {
                self.status = format!(
                    "QR settings saved · {} · {}px · {} on {}",
                    self.qr_settings.format.label(),
                    self.qr_settings.size,
                    self.qr_settings.fg,
                    self.qr_settings.bg,
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // CSV import
    // ------------------------------------------------------------------

    fn open_import(&mut self) {
        self.import = Some(ImportState::new());
        self.screen = Screen::Import;
        self.status = String::new();
    }

    fn close_import(&mut self) {
        self.import = None;
        self.screen = Screen::LinkList;
        self.reload("Refreshing…", self.page);
    }

    fn on_import_key(&mut self, key: KeyEvent) {
        let stage = match self.import.as_ref().map(|s| &s.stage) {
            Some(ImportStage::Browse) => 0,
            Some(ImportStage::Preview(_)) => 1,
            Some(ImportStage::Running(_)) => 2,
            Some(ImportStage::Done(_)) => 3,
            None => return,
        };
        match stage {
            0 => self.import_browse_key(key),
            1 => self.import_preview_key(key),
            2 => {} // running — ignore input
            _ => self.import_done_key(key),
        }
    }

    fn import_browse_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.close_import(),
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(s) = self.import.as_mut() {
                    s.browser.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(s) = self.import.as_mut() {
                    s.browser.move_down();
                }
            }
            KeyCode::Left | KeyCode::Backspace => {
                if let Some(s) = self.import.as_mut() {
                    if let Some(parent) = s.browser.dir.parent().map(Path::to_path_buf) {
                        s.browser.open_dir(parent);
                    }
                }
            }
            KeyCode::Char('t') => self.import_write_template(),
            KeyCode::Enter => self.import_enter_selection(),
            _ => {}
        }
    }

    fn import_enter_selection(&mut self) {
        let ws = self.workspace_id;
        let Some(s) = self.import.as_mut() else { return };
        let Some(entry) = s.browser.selected() else { return };
        if entry.is_dir {
            let dir = entry.path.clone();
            s.browser.open_dir(dir);
            return;
        }
        let path = entry.path.clone();
        match crate::forms::import::parse_csv(&path, ws) {
            Ok(parsed) => {
                s.message = None;
                s.stage = ImportStage::Preview(parsed);
            }
            Err(e) => s.message = Some(format!("Parse error: {e}")),
        }
    }

    fn import_write_template(&mut self) {
        let Some(s) = self.import.as_mut() else { return };
        let path = s.browser.dir.join("linkly-import-template.csv");
        s.message = Some(match crate::forms::import::write_template(&path) {
            Ok(()) => format!("Wrote template to {}", path.display()),
            Err(e) => format!("Template error: {e}"),
        });
        s.browser.refresh();
    }

    fn import_preview_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') => self.import_confirm(),
            KeyCode::Esc | KeyCode::Char('n') => {
                if let Some(s) = self.import.as_mut() {
                    s.stage = ImportStage::Browse;
                }
            }
            _ => {}
        }
    }

    fn import_confirm(&mut self) {
        let (reqs, dir) = {
            let Some(s) = self.import.as_mut() else { return };
            let ImportStage::Preview(parsed) = &s.stage else { return };
            let reqs = parsed.valid_requests();
            let dir = parsed
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let total = reqs.len();
            s.stage = ImportStage::Running(Progress {
                total,
                done: 0,
                ok: 0,
                failed: 0,
            });
            (reqs, dir)
        };
        let Some(client) = self.client.clone() else { return };
        let tx = self.tx.clone();
        self.status = "Importing…".to_string();
        self.loading = true;
        tokio::spawn(async move {
            run_import(client, tx, reqs, dir).await;
        });
    }

    fn import_awaiting_qr(&self) -> bool {
        if let Some(s) = &self.import {
            if let ImportStage::Done(sum) = &s.stage {
                return sum.qr_done.is_none() && !sum.new_links.is_empty();
            }
        }
        false
    }

    fn import_done_key(&mut self, key: KeyEvent) {
        let awaiting = self.import_awaiting_qr();
        match key.code {
            KeyCode::Char('y') if awaiting => self.import_generate_qr(),
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('n') => {
                self.close_import();
            }
            _ => {}
        }
    }

    fn import_generate_qr(&mut self) {
        let links: Vec<NewLink> = match &self.import {
            Some(s) => match &s.stage {
                ImportStage::Done(sum) => sum.new_links.clone(),
                _ => return,
            },
            None => return,
        };
        let ws = self.workspace_id;
        let settings = self.qr_settings.clone();
        let tx = self.tx.clone();
        self.status = "Generating QR codes…".to_string();
        self.loading = true;
        tokio::spawn(async move {
            let dir = crate::qr::output_dir(ws);
            let mut count = 0usize;
            for l in &links {
                if let Some(url) = l.full_url.as_deref().filter(|u| !u.is_empty()) {
                    let fname = crate::qr::file_name(
                        l.id,
                        l.slug.as_deref(),
                        l.name.as_deref(),
                        settings.format,
                    );
                    if crate::qr::write_qr(url, &dir.join(fname), &settings).is_ok() {
                        count += 1;
                    }
                }
            }
            let _ = tx.send(AsyncMsg::QrExported {
                count,
                dir: dir.display().to_string(),
            });
        });
    }

    fn load_workspaces(&self) {
        let Some(client) = self.client.clone() else { return };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            if let Ok(workspaces) = client.list_workspaces().await {
                let _ = tx.send(AsyncMsg::WorkspacesLoaded(workspaces));
            }
        });
    }

    /// Record that the active workspace was just used: move it to the front of
    /// the cache (most-recently-used first) and persist. `name_hint` updates the
    /// stored name when known; otherwise the existing name is kept.
    fn record_workspace_use(&mut self, name_hint: Option<String>) {
        let existing = self
            .cached_workspaces
            .iter()
            .find(|w| w.id == self.workspace_id);
        let name = name_hint
            .or_else(|| existing.map(|w| w.name.clone()))
            .unwrap_or_else(|| format!("Workspace {}", self.workspace_id));
        // Preserve any stored key across the move-to-front.
        let api_key = existing.and_then(|w| w.api_key.clone());
        self.cached_workspaces.retain(|w| w.id != self.workspace_id);
        self.cached_workspaces.insert(
            0,
            CachedWorkspace {
                id: self.workspace_id,
                name,
                api_key,
            },
        );
        crate::config::save_workspaces(&self.cached_workspaces);
    }

    /// Whether the active workspace already has the current key stored.
    fn active_key_stored(&self) -> bool {
        self.cached_workspaces
            .iter()
            .find(|w| w.id == self.workspace_id)
            .and_then(|w| w.api_key.as_deref())
            == Some(self.current_key.as_str())
    }

    /// Store the current session's API key against the active workspace.
    fn store_active_key(&mut self) {
        if let Some(w) = self
            .cached_workspaces
            .iter_mut()
            .find(|w| w.id == self.workspace_id)
        {
            w.api_key = Some(self.current_key.clone());
            crate::config::save_workspaces(&self.cached_workspaces);
        }
    }

    // ------------------------------------------------------------------
    // Async result handling
    // ------------------------------------------------------------------

    pub fn on_async(&mut self, msg: AsyncMsg) {
        match msg {
            AsyncMsg::LinksLoaded(resp) => {
                self.links = resp.links;
                self.total_pages = resp.total_pages.max(1);
                self.total_entries = resp.total_entries;
                if resp.page_number > 0 {
                    self.page = resp.page_number;
                }
                self.loading = false;
                self.status = format!(
                    "{} links (page {}/{})",
                    self.total_entries, self.page, self.total_pages
                );
                if self.links.is_empty() {
                    self.list_state.select(None);
                } else {
                    let i = self
                        .list_state
                        .selected()
                        .unwrap_or(0)
                        .min(self.links.len() - 1);
                    self.list_state.select(Some(i));
                }
                // A successful response verifies the key and workspace id, so
                // it's now safe to cache the workspace (and offer to store the
                // key unless it's already saved for this workspace).
                if self.verify_pending {
                    self.verify_pending = false;
                    let name_hint = self.pending_ws_name.take();
                    self.record_workspace_use(name_hint);
                    if !self.current_key.is_empty() && !self.active_key_stored() {
                        self.store_prompt = true;
                    }
                }
            }
            AsyncMsg::LinkDetailLoaded(v) => {
                self.editor = Some(LinkEditor::from_value(&v, self.workspace_id));
                self.loading = false;
                self.status = String::new();
            }
            AsyncMsg::DomainsLoaded(domains) => {
                self.domains = domains;
            }
            AsyncMsg::WorkspacesLoaded(workspaces) => {
                let name = workspaces
                    .iter()
                    .find(|w| w.id == self.workspace_id)
                    .map(|w| w.name.clone())
                    .filter(|n| !n.is_empty());
                if self.cached_workspaces.iter().any(|w| w.id == self.workspace_id) {
                    // Already cached (verified): refresh its name.
                    self.record_workspace_use(name);
                } else {
                    // Not cached yet — remember the name so it's applied once the
                    // workspace verifies via a successful link load.
                    self.pending_ws_name = name;
                }
            }
            AsyncMsg::LinkCreated(new) => {
                self.screen = Screen::LinkList;
                self.status = "Link created — refreshing…".to_string();
                self.loading = true;
                self.load_links(self.page);
                // Offer a QR code for the freshly created link (Enter to export
                // in the dialog, Esc to skip).
                if let Some(url) = new.full_url.clone().filter(|u| !u.is_empty()) {
                    self.open_qr_settings(Some(QrExportTarget::Single(QrSingle {
                        id: new.id,
                        slug: new.slug,
                        name: new.name,
                        url,
                    })));
                }
            }
            AsyncMsg::LinkUpdated => {
                self.loading = false;
                // The saved values become the new baseline, so the editor is no
                // longer "dirty" and Esc won't prompt to save again.
                let exit = if let Some(e) = self.editor.as_mut() {
                    e.mark_saved();
                    e.exit_after_save
                } else {
                    false
                };
                if exit {
                    self.exit_detail();
                } else {
                    self.status = "Link saved ✓".to_string();
                }
            }
            AsyncMsg::QrExported { count, dir } => {
                self.loading = false;
                self.status = format!("Exported {count} QR code(s) to {dir}/");
                // If this was the post-import QR run, record it on the summary.
                if let Some(s) = self.import.as_mut() {
                    if let ImportStage::Done(sum) = &mut s.stage {
                        sum.qr_done = Some(count);
                        sum.qr_dir = Some(dir);
                    }
                }
            }
            AsyncMsg::ImportProgress { done, ok, failed } => {
                if let Some(s) = self.import.as_mut() {
                    if let ImportStage::Running(p) = &mut s.stage {
                        p.done = done;
                        p.ok = ok;
                        p.failed = failed;
                    }
                }
            }
            AsyncMsg::ImportDone {
                ok,
                failed,
                success_path,
                failure_path,
                new_links,
            } => {
                self.loading = false;
                self.status = format!("Imported {ok} link(s), {failed} failed");
                if let Some(s) = self.import.as_mut() {
                    s.stage = ImportStage::Done(Summary {
                        ok,
                        failed,
                        success_path: success_path.map(PathBuf::from),
                        failure_path: failure_path.map(PathBuf::from),
                        new_links,
                        qr_done: None,
                        qr_dir: None,
                    });
                }
            }
            AsyncMsg::Error(e) => {
                self.loading = false;
                if self.screen == Screen::Auth {
                    self.auth.error = Some(e.clone());
                }
                self.status = format!("Error: {e}");
            }
        }
    }
}

/// Create one link, retrying with backoff on rate-limit (HTTP 429).
async fn submit_one(client: &LinklyClient, req: &CreateLinkRequest) -> Result<Value, String> {
    let mut delay = Duration::from_secs(1);
    for attempt in 0..4 {
        match client.create_link(req).await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("429") && attempt < 3 {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(8));
                    continue;
                }
                return Err(msg);
            }
        }
    }
    Err("rate limited".to_string())
}

/// Submit all parsed rows sequentially, reporting progress, then write the
/// success/failure CSVs next to the input file.
async fn run_import(
    client: LinklyClient,
    tx: UnboundedSender<AsyncMsg>,
    reqs: Vec<(u64, CreateLinkRequest)>,
    dir: PathBuf,
) {
    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut new_links: Vec<NewLink> = Vec::new();
    let mut failures: Vec<(u64, String)> = Vec::new();

    for (i, (line, req)) in reqs.into_iter().enumerate() {
        match submit_one(&client, &req).await {
            Ok(v) => {
                ok += 1;
                new_links.push(NewLink::from_response(&v));
            }
            Err(e) => {
                failed += 1;
                failures.push((line, e));
            }
        }
        let _ = tx.send(AsyncMsg::ImportProgress {
            done: i + 1,
            ok,
            failed,
        });
    }

    let success_path = (!new_links.is_empty())
        .then(|| crate::forms::import::write_success(&dir, &new_links).ok())
        .flatten();
    let failure_path = (!failures.is_empty())
        .then(|| crate::forms::import::write_failures(&dir, &failures).ok())
        .flatten();

    let _ = tx.send(AsyncMsg::ImportDone {
        ok,
        failed,
        success_path: success_path.map(|p| p.display().to_string()),
        failure_path: failure_path.map(|p| p.display().to_string()),
        new_links,
    });
}

/// Page through every link in the workspace and write a QR PNG for each.
async fn export_all_qr(
    client: &LinklyClient,
    workspace_id: i64,
    search: &str,
    sort_by: &str,
    sort_dir: &str,
    settings: &crate::qr::QrSettings,
) -> AsyncMsg {
    let dir = crate::qr::output_dir(workspace_id);
    let mut count = 0usize;
    let mut page = 1i64;
    loop {
        let resp = match client
            .list_links(workspace_id, page, PAGE_SIZE, search, sort_by, sort_dir)
            .await
        {
            Ok(r) => r,
            Err(e) => return AsyncMsg::Error(format!("QR export failed: {e}")),
        };
        for l in &resp.links {
            if let Some(url) = l.full_url.as_deref().filter(|u| !u.is_empty()) {
                let fname =
                    crate::qr::file_name(l.id, l.slug.as_deref(), l.name.as_deref(), settings.format);
                if crate::qr::write_qr(url, &dir.join(fname), settings).is_ok() {
                    count += 1;
                }
            }
        }
        if page >= resp.total_pages.max(1) {
            break;
        }
        page += 1;
    }
    AsyncMsg::QrExported {
        count,
        dir: dir.display().to_string(),
    }
}
