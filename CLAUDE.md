# LiteSkill VR

Structured notebook for reverse-engineering research. A Tauri v2 desktop app
(React + TypeScript frontend, Rust backend) that embeds an MCP server so AI
agents document findings into a single `.lsvr` SQLite file alongside the
researcher.

**License**: Apache-2.0 · **Node**: ≥22 · **Rust**: ≥1.77

This file is operating guidance for working in this repo: the non-obvious
conventions, the real (hook/CI-enforced) commands, and where things live. It is
deliberately not an API dump — for the data model and MCP tool surface read the
specs under `spec/`, linked below.

`AGENTS.md` is a symlink to this file, so Codex and other agent tools read the
same guidance. Edit `CLAUDE.md`; never edit them out of sync.

## Repo layout

| Path                        | What it is                                                                                                                                                       |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/`                      | React 19 + TypeScript frontend (Vite). Entry `src/main.tsx`, root `src/App.tsx`.                                                                                 |
| `src/components/`           | UI views — Dashboard, ItemDetail, ConnectionMap, Sidebar, TabBar, StatusBar.                                                                                     |
| `src/lib/`                  | `store.ts` (Zustand), `ipc.ts` (Tauri invoke), `types.ts` (shared types).                                                                                        |
| `src-tauri/`                | Rust backend (the Tauri crate, `liteskill_vr_lib`).                                                                                                              |
| `src-tauri/src/db/`         | SQLite layer — one file per entity (`items.rs`, `notes.rs`, `ioi.rs`, `connections.rs`, …) + `migrations.rs`, `models.rs`, `search.rs`. **Pure: no Tauri deps.** |
| `src-tauri/src/mcp/`        | MCP server — `tools.rs`, `handlers.rs`, `server.rs`. **Pure: no Tauri deps.**                                                                                    |
| `src-tauri/src/bin/mcp.rs`  | Headless `liteskillvr-mcp` binary entry point.                                                                                                                   |
| `src-tauri/src/commands.rs` | Tauri IPC commands (see Frontend gotcha below).                                                                                                                  |
| `src-tauri/src/lib.rs`      | GUI wiring + `MCP_PORT` (27182).                                                                                                                                 |
| `spec/`                     | **Design source of truth** — overview, architecture, data-model, mcp, ui, file-formats, security.                                                                |
| `docs/`                     | Rendered static doc site (generated/hand-tuned; Prettier-ignored). Don't hand-fix here for design changes — edit `spec/`.                                        |
| `scripts/`                  | `release.sh`, `sync-version.sh`, `check-docs.mjs`.                                                                                                               |
| `e2e/`                      | WebdriverIO end-to-end tests.                                                                                                                                    |

## Task runner — the single entry point

`Taskfile.yml` ([taskfile.dev](https://taskfile.dev)) is the canonical way to
invoke every workflow. CI and the git hooks call these tasks rather than raw
`pnpm`/`cargo`, so prefer `task ...` — raw commands work but the task is the
contract. Run `task` to list everything.

| Command                             | Does                                                                  |
| ----------------------------------- | --------------------------------------------------------------------- |
| `task setup`                        | Install JS deps + git hooks (lefthook).                               |
| `task dev`                          | Run the Tauri app in dev mode.                                        |
| `task check`                        | **Full gate — run this before declaring work done.** Frontend + Rust. |
| `task check:fe` / `task check:rust` | Just one side (these are the two CI jobs).                            |
| `task lint` / `task fmt`            | eslint + clippy / prettier + cargo fmt.                               |
| `task test`                         | vitest + cargo test.                                                  |
| `task build`                        | Release app + installers. `task build:mcp` for the headless binary.   |
| `task docs:check`                   | Anti-rot check for this file (task refs + links resolve).             |
| `task release:patch\|minor\|major`  | Bump version, sync, commit, tag.                                      |

## Rust: features & binaries — read this before touching `src-tauri/`

There are **two binaries** from one crate, gated by the `gui` Cargo feature
(default on):

- `liteskill-vr` — the desktop app. **Requires `gui`** (pulls Tauri +
  WebKitGTK). Plain `cargo build` builds this.
- `liteskillvr-mcp` (`src-tauri/src/bin/mcp.rs`) — the headless MCP server. Build it with
  `--no-default-features` so it links **none** of Tauri/WebKitGTK:
  `cargo build --release --bin liteskillvr-mcp --no-default-features`
  (or just `task build:mcp`).

This only works because `db/` and `mcp/` are **pure modules with no Tauri
dependency**. Keep it that way — if you make either depend on Tauri, the
headless binary stops compiling. CI lints both feature sets
(`task rs:clippy` and `task rs:clippy:headless`); do the same locally.

Lint policy lives in the `[lints]` table in `src-tauri/Cargo.toml` (clippy
pedantic + nursery at `warn`), **not** in a cargo `config.toml` rustflags block.
Warnings are not
denied during plain builds; CI and hooks run clippy with `-D warnings` (via
`task rs:clippy`) to make them hard failures. Dependency audit is `cargo deny`
(`deny.toml`), run by `task rs:deny`.

Schema changes go through `src-tauri/src/db/migrations.rs`.

## Frontend: the UI is read-only over IPC

Non-obvious and important: the desktop UI **does not mutate data through Tauri
IPC**. There is exactly one command — `project_snapshot` (`src-tauri/src/commands.rs`)
— which returns the whole project as JSON; `src/lib/ipc.ts` exposes it as
`getSnapshot()`. `src/App.tsx` fetches the snapshot on mount and re-fetches
whenever the backend emits the **`db-changed`** event. The Zustand store
(`src/lib/store.ts`) holds that snapshot.

> All writes happen through the **MCP server**, not IPC. When you add a feature
> that creates or edits research data, it belongs in the MCP tool layer
> (`src-tauri/src/mcp/`), and the UI picks it up via the next snapshot.

(Note: `spec/architecture.md` still describes a richer multi-command IPC
contract and an `entity_changed` event — that's design intent that the code has
since simplified. Trust the code; the real event is `db-changed`.)

Frontend conventions, all enforced by `task check:fe`:

- Strict TypeScript (`tsconfig.json` has every strict flag on, incl.
  `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`). Import via the
  `@/` alias for `src/`.
- ESLint is type-aware strict (`strictTypeChecked`) + React/hooks/jsx-a11y +
  import ordering; exported functions need explicit return types; inline type
  imports (`import { type Foo }`). Prettier owns formatting.
- `knip` fails on unused files/deps/exports.

## Data & MCP model (brief — details in `spec/`)

- **One `.lsvr` file = one project** (SQLite, FTS5 full-text search).
- Entities: items, notes, items-of-interest, connections, tags, connection
  types. See `spec/data-model.md`.
- **Author identity is never passed per call.** Over HTTP it comes from the
  `X-LiteSkill-Author` header (default `anonymous-agent`); over stdio it's
  `stdio-agent`; UI writes use the OS username. See `spec/mcp.md`.
- **Registered vocabularies**: tags and connection types must exist before use.
- **Batches are transactional** — any invalid entry rejects the whole batch.
- MCP server binds `127.0.0.1` only, port `MCP_PORT` in `src-tauri/src/lib.rs`.

## Testing

- Frontend unit tests: `task fe:test` (vitest, `src/**/*.test.tsx`, jsdom).
- Rust tests: `task rs:test` (`src-tauri/tests/mcp_integration.rs` drives the
  MCP surface end-to-end).
- E2E: `task fe:test:e2e` (WebdriverIO) — **requires a release build first**
  (`task build`) and `WebKitWebDriver`.

## Releasing

`task release:patch|minor|major` bumps `package.json`, runs
`scripts/sync-version.sh` to mirror the version into `src-tauri/Cargo.toml` and
`tauri.conf.json`, then commits and tags. Pushing the tag drives the release
workflow.

## Further docs

- `spec/` — design source of truth (start at `spec/overview.md`).
- `docs/` — rendered guide site (`task docs:serve`), also published to GitHub
  Pages.
- Project home: <https://liteskill.io>
