# linkly-tui

A fast, keyboard-driven terminal UI for managing [Linkly](https://linklyhq.com)
short links — built in Rust on [ratatui](https://ratatui.rs).

Authenticate once, then browse your workspace's links, inspect any link in
detail, and create new links (with every option Linkly supports, including a
custom domain) without leaving the terminal.

```
  ██╗     ██╗███╗   ██╗██╗  ██╗██╗  ██╗   ██╗
  ██║     ██║████╗  ██║██║ ██╔╝██║  ╚██╗ ██╔╝
  ██║     ██║██╔██╗ ██║█████╔╝ ██║   ╚████╔╝
  ██║     ██║██║╚██╗██║██╔═██╗ ██║    ╚██╔╝
  ███████╗██║██║ ╚████║██║  ██╗███████╗██║
  ╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝╚═╝
            terminal client
```

## Features

- **Secure sign-in** — enter your Linkly **API key** (rendered masked) and
  **workspace ID** on startup. Credentials are **never written to disk**.
- **Browse links** — a paginated table of every link in the workspace with live
  click stats (total / 30-day / today) and enabled status. The panel title always
  shows the current sort and page (`page 1/3 · 240 total`). Search, sort by any
  column (asc/desc), page, and refresh on demand.
- **Link details** — the full record for any selected link, with values neatly
  aligned and colour-coded.
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

## Keybindings

| Screen  | Keys |
|---------|------|
| Sign in | `Tab` switch field · `Enter` continue · `Esc` quit |
| List    | `↑/↓` move · `Enter` details · `c` create · `/` search · `s` sort · `n`/`p` next/prev page · `r` refresh · `q` quit |
| Sort    | `↑/↓` field · `d`/`←→` direction · `Enter` apply · `Esc` cancel |
| Detail  | `↑/↓` scroll · `Esc` back |
| Create  | `Tab`/`↑↓` move field · `Space` toggle boolean · `Ctrl-A` show/hide advanced · `Enter` open domain picker / save on **Submit** · `Esc` cancel |

## Architecture

API calls run on Tokio tasks and report results back to a non-blocking UI loop
over an `mpsc` channel (`AsyncMsg`), so the interface never freezes on the
network.

```
src/
  main.rs            terminal setup/teardown, Tokio runtime, event loop
  app.rs             App state machine (Screen enum), event dispatch, async orchestration
  config.rs          credential env prefill (no disk persistence)
  api/
    client.rs        LinklyClient — one async method per endpoint
    models.rs        serde models (CreateLinkRequest is the shared write contract)
  forms/
    create_form.rs   create-form state + pure build() -> CreateLinkRequest
  ui/
    mod.rs           shared theme, status bar, panel/layout helpers
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
