//! Application state machine, event dispatch, and async orchestration.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::api::models::{LinkSummary, ListLinksResponse};
use crate::api::LinklyClient;
use crate::forms::{CreateForm, DomainSelector, Field};

pub const PAGE_SIZE: i64 = 100;

/// Which screen is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Auth,
    LinkList,
    LinkDetail,
    CreateLink,
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
    LinkCreated,
    Error(String),
}

/// The two-field credential prompt shown on startup.
#[derive(Default)]
pub struct AuthState {
    pub api_key: Input,
    pub workspace_id: Input,
    /// 0 = api key, 1 = workspace id.
    pub focus: usize,
    pub error: Option<String>,
}

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
    pub status: String,
    pub loading: bool,

    pub auth: AuthState,
    pub client: Option<LinklyClient>,
    pub workspace_id: i64,

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

    // Detail state.
    pub detail: Option<Value>,
    pub detail_scroll: u16,

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
        };
        Self {
            screen: Screen::Auth,
            should_quit: false,
            status: String::new(),
            loading: false,
            auth,
            client: None,
            workspace_id: 0,
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
            detail: None,
            detail_scroll: 0,
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
        match self.screen {
            Screen::Auth => self.on_auth_key(key),
            Screen::LinkList => self.on_list_key(key),
            Screen::LinkDetail => self.on_detail_key(key),
            Screen::CreateLink => self.on_create_key(key),
        }
    }

    fn on_auth_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab | KeyCode::Down => self.auth.focus = (self.auth.focus + 1) % 2,
            KeyCode::Up | KeyCode::BackTab => self.auth.focus = (self.auth.focus + 1) % 2,
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
        let ws = self.auth.workspace_id.value().trim().to_string();
        if key.is_empty() {
            self.auth.error = Some("API key is required".to_string());
            return;
        }
        let Ok(ws_id) = ws.parse::<i64>() else {
            self.auth.error = Some("Workspace ID must be a number".to_string());
            return;
        };
        self.auth.error = None;
        self.client = Some(LinklyClient::new(key));
        self.workspace_id = ws_id;
        self.screen = Screen::LinkList;
        self.page = 1;
        self.status = "Loading links…".to_string();
        self.loading = true;
        self.load_links();
        self.load_domains();
    }

    fn on_list_key(&mut self, key: KeyEvent) {
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
                    self.reload("Sorting…");
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
                    self.reload("Searching…");
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
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_prev(),
            KeyCode::Enter => self.open_detail(),
            KeyCode::Char('c') => self.open_create(),
            KeyCode::Char('r') => self.reload("Refreshing…"),
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
                if self.page < self.total_pages {
                    self.page += 1;
                    self.reload("Loading…");
                }
            }
            KeyCode::Char('p') => {
                if self.page > 1 {
                    self.page -= 1;
                    self.reload("Loading…");
                }
            }
            _ => {}
        }
    }

    fn on_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
                self.screen = Screen::LinkList;
                self.detail = None;
            }
            KeyCode::Down | KeyCode::Char('j') => self.detail_scroll = self.detail_scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.detail_scroll = self.detail_scroll.saturating_sub(1),
            _ => {}
        }
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
                Ok(_) => AsyncMsg::LinkCreated,
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
        self.detail = None;
        self.detail_scroll = 0;
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

    fn reload(&mut self, status: &str) {
        self.status = status.to_string();
        self.loading = true;
        self.load_links();
    }

    // ------------------------------------------------------------------
    // Async task spawners
    // ------------------------------------------------------------------

    fn load_links(&self) {
        let Some(client) = self.client.clone() else { return };
        let tx = self.tx.clone();
        let ws = self.workspace_id;
        let page = self.page;
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
            }
            AsyncMsg::LinkDetailLoaded(v) => {
                self.detail = Some(v);
                self.loading = false;
                self.status = "Link details".to_string();
            }
            AsyncMsg::DomainsLoaded(domains) => {
                self.domains = domains;
            }
            AsyncMsg::LinkCreated => {
                self.screen = Screen::LinkList;
                self.status = "Link created — refreshing…".to_string();
                self.loading = true;
                self.load_links();
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
