# linkly-tui

A fast, keyboard-driven terminal UI for managing [Linkly](https://linklyhq.com)
short links — built in Rust on [ratatui](https://ratatui.rs).

Authenticate once, then browse your workspace's links, inspect any link in
detail, and create new links (with every option Linkly supports, including a
custom domain) without leaving the terminal.

```
  ██╗     ██╗███╗   ██╗██╗  ██╗██╗  ██╗   ██╗████████╗██╗   ██╗██╗
  ██║     ██║████╗  ██║██║ ██╔╝██║  ╚██╗ ██╔╝╚══██╔══╝██║   ██║██║
  ██║     ██║██╔██╗ ██║█████╔╝ ██║   ╚████╔╝    ██║   ██║   ██║██║
  ██║     ██║██║╚██╗██║██╔═██╗ ██║    ╚██╔╝     ██║   ██║   ██║██║
  ███████╗██║██║ ╚████║██║  ██╗███████╗██║      ██║   ╚██████╔╝██║
  ╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝╚═╝      ╚═╝    ╚═════╝ ╚═╝
```

## Features

- **Workspace picker & cache** — remembered workspaces (id + name) are listed on
  startup, most-recently-used first; pick one or add a new one. Workspaces with a
  stored key are marked `🔑 key saved`.
- **Sign-in & optional key storage** — enter your Linkly **API key** (rendered
  masked). For a new workspace you also provide its ID; for a cached one the ID is
  known (and a stored key is pre-filled). After the key verifies, you're *offered*
  the choice to store it for that workspace — opt-in, never required. **Stored
  keys are saved in plaintext**, which is a security risk you're warned about
  before storing (see below).
- **Browse links** — a paginated table of every link in the workspace with live
  click stats (total / 30-day / today) and enabled status. The panel title always
  shows the current sort and page (`page 1/3 · 240 total`). Search, sort by any
  column (asc/desc), page, and refresh on demand.
- **View & edit links** — open any link to see its fields in a navigable list
  (the current line is highlighted; no free-scrolling). `Enter` edits the
  selected field, `Esc` leaves edit mode. Changed fields are marked, and on the
  way out you're asked to save if there are unsaved changes; `s` saves at any
  time. Once saved, the new values are the baseline — leaving no longer prompts.
- **QR codes** — press `Q` to export a QR code for a link's short URL: in the
  detail view for the selected link, or in the list to batch-export **every**
  link in the workspace. `Q` opens a dialog to choose **format** (PNG default,
  SVG, JPEG), **size**, and **fg/bg colours** before exporting (`Enter` to
  export, `Esc` to cancel). Files land in `./linkly-qr/<workspace-id>/`, named by
  link id + slug. Choices persist to `~/.config/linkly-tui/qr.json`; press `o` to
  edit those defaults without exporting. (Linkly's API has no QR endpoint; codes
  are rendered locally from each link's short URL.)
- **Create links** — a form exposing the full Linkly option set. Core fields are
  always visible; `Ctrl-A` reveals advanced fields (OG tags, UTM parameters,
  tracking pixels, cloaking, bot-blocking, custom head/body tags, …). The custom
  **domain is chosen from your workspace's domains**, and the list
  **auto-refreshes** after a successful create.

## Requirements

- [Rust](https://rustup.rs) 1.80+ (2021 edition) and Cargo
- A real terminal (the app takes over the screen; it can't run with a piped or
  absent TTY)
- A Linkly account with an [API key](https://app.linklyhq.com) and a workspace ID

## Install & run

```bash
# from the project root
cargo run --release
```

On first launch you'll be prompted for your API key and workspace ID. To build a
standalone binary:

```bash
cargo build --release
./target/release/linkly-tui
```

### Environment variables (optional)

The sign-in prompt can be pre-filled from the environment. You still confirm with
`Enter` — nothing is read or stored silently.

| Variable               | Purpose                       |
|------------------------|-------------------------------|
| `LINKLY_API_KEY`       | Pre-fills the API key field   |
| `LINKLY_WORKSPACE_ID`  | Pre-fills the workspace ID    |

```bash
LINKLY_API_KEY=sk_… LINKLY_WORKSPACE_ID=42 cargo run --release
```

### Workspace cache & stored keys

Known workspaces are stored at `~/.config/linkly-tui/workspaces.json` (honouring
`XDG_CONFIG_HOME`). The id and name are always cached. An API key is cached only
if you explicitly accept the "Store API key?" prompt after signing in.

> ⚠️ **Security warning:** stored keys are written in **plaintext**. Anyone who
> can read that file (other local users, backups, synced dotfiles, etc.) can use
> your Linkly account. Only store keys on a machine you trust, and prefer the
> `LINKLY_API_KEY` env var or entering the key each time if in doubt.

Press `d` on a workspace in the picker to forget it, which also deletes any key
stored for it. Deleting the file removes everything.

## Keybindings

| Screen  | Keys |
|---------|------|
| Workspaces | `↑/↓` select · `Enter` continue · `d` forget (+ stored key) · `Esc`/`q` quit |
| Sign in | `Tab` switch field · `Enter` continue · `Esc` back/quit |
| Store key? | `s` store · `n`/`Esc` not now |
| List    | `↑/↓` move · `Enter` details · `c` create · `Q` export QR (workspace) · `o` QR defaults · `/` search · `s` sort · `n`/`p` page · `r` refresh · `Esc` workspaces · `q` quit |
| QR dialog | `↑/↓` field · `←/→` format · type to edit size/colours · `Enter` export/save · `Esc` cancel |
| Sort    | `↑/↓` field · `d`/`←→` direction · `Enter` apply · `Esc` cancel |
| Detail  | `↑/↓` move field · `Enter` edit / toggle · `s` save · `Q` export QR · `Esc` back (prompts if unsaved) |
| Editing | type to edit · `Enter`/`Esc` finish editing the field |
| Save?   | `s` save · `d` discard · `Esc` cancel |
| Create  | `Tab`/`↑↓` move field · `Space` toggle boolean · `Ctrl-A` show/hide advanced · `Enter` open domain picker / save on **Submit** · `Esc` cancel |

## Architecture

API calls run on Tokio tasks and report results back to a non-blocking UI loop
over an `mpsc` channel (`AsyncMsg`), so the interface never freezes on the
network.

```
src/
  main.rs            terminal setup/teardown, Tokio runtime, event loop
  app.rs             App state machine (Screen enum), event dispatch, async orchestration
  config.rs          credential env prefill + workspace cache (ids/names, opt-in keys)
  qr.rs              local QR-code generation (png/svg/jpeg, size/colour), single + batch
  api/
    client.rs        LinklyClient — one async method per endpoint
    models.rs        serde models (CreateLinkRequest is the shared write contract)
  forms/
    create_form.rs   create-form state + pure build() -> CreateLinkRequest
    edit_form.rs     link editor state (dirty tracking, update payload, save baseline)
  ui/
    mod.rs           shared theme, banner, status bar, panel/layout helpers
    workspace.rs     startup workspace picker
    auth.rs          sign-in screen
    list.rs          links table
    detail.rs        single-link detail view
    create.rs        create form + domain picker popup
```

### Tech stack

| Concern        | Crate |
|----------------|-------|
| TUI rendering  | `ratatui` + `crossterm` |
| Async runtime  | `tokio` |
| HTTP client    | `reqwest` (rustls) |
| Serialization  | `serde` / `serde_json` |
| Text input     | `tui-input` |
| Errors         | `anyhow` |

The Linkly API is documented in [`api-1.json`](./api-1.json) (OpenAPI 3). The
client authenticates via the `api_key` query parameter against
`https://api.linklyhq.com`.

## Development

```bash
cargo build            # compile
cargo test             # unit tests (request building & serialization)
cargo clippy --all-targets   # lint (kept warning-free)
```

## Roadmap

- **CSV batch import** — the groundwork is already in place:
  - `CreateLinkRequest` (`api/models.rs`) is the single write contract, with
    `skip_serializing_if` on every optional field, so only set values are sent.
  - `CreateForm::build()` is a pure `state -> CreateLinkRequest` function.
  - The `csv` crate is already a dependency.

  A future `import` module will read a CSV (column headers = field names), map
  each row to a `CreateLinkRequest`, and submit rows sequentially with a progress
  view — reusing `LinklyClient::create_link` and the existing models, with no
  refactor.

## License

Not yet specified.
