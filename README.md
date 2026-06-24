# linkly-tui

A terminal UI for managing [Linkly](https://linklyhq.com) short links, built on
[ratatui](https://ratatui.rs).

## Features

- **Authentication** — on startup you enter your Linkly **API key** (masked) and
  **workspace ID**. Credentials are never written to disk. The prompt can be
  pre-filled from the `LINKLY_API_KEY` / `LINKLY_WORKSPACE_ID` environment
  variables (you still confirm with Enter).
- **List links** — paginated table of every link in the workspace with click
  stats (total / 30-day / today) and enabled status. Search, paging and refresh.
- **Create a link** — a form with all the API's options. Core fields are always
  visible; `Ctrl-A` reveals advanced fields (OG tags, UTM, pixels, cloaking,
  bots, …). The custom **domain is picked from the workspace's domains**. The
  list auto-refreshes after a successful create.
- **Link details** — full record for the selected link.

## Usage

```bash
cargo run
```

Then sign in at the prompt.

### Keys

| Screen  | Keys |
|---------|------|
| Sign in | `Tab` switch field · `Enter` continue · `Esc` quit |
| List    | `↑/↓` move · `Enter` details · `c` create · `/` search · `n`/`p` page · `r` refresh · `q` quit |
| Detail  | `↑/↓` scroll · `Esc` back |
| Create  | `Tab` move · `Space` toggle · `Ctrl-A` advanced · `Enter` (on *Submit*) save · `Esc` cancel |

## Architecture

```
src/
  main.rs            terminal setup/teardown, tokio runtime, event loop
  app.rs             App state machine (Screen enum), event dispatch, async orchestration
  config.rs          credential env prefill (no disk persistence)
  api/
    client.rs        LinklyClient — one async method per endpoint
    models.rs        serde models (CreateLinkRequest is the shared write contract)
  forms/
    create_form.rs   create-form state + pure build() -> CreateLinkRequest
  ui/                per-screen rendering (auth, list, detail, create)
```

API calls run on tokio tasks and report results back to the non-blocking UI loop
over an `mpsc` channel (`AsyncMsg`).

## Planned: CSV batch import

The groundwork is in place:

- `CreateLinkRequest` (in `api/models.rs`) is the single write contract, with
  `skip_serializing_if` on every optional field, so only set values are sent.
- `CreateForm::build()` is a pure `state -> CreateLinkRequest` function.
- The `csv` crate is already a dependency.

A future `import` module will read a CSV (column headers = field names), map each
row to a `CreateLinkRequest`, and submit rows sequentially with a progress view —
reusing `LinklyClient::create_link` and the existing models, with no refactor.
