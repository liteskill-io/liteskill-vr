# Architecture

## High-Level Diagram

```
┌─────────────────────────────────────────────────┐
│               LiteSkill VR (Tauri)              │
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │           Frontend (React/TS)             │  │
│  │                                           │  │
│  │  Tabbed Views · Search · Connection Map   │  │
│  │  Item Browser · Notes · Tags              │  │
│  └─────────────────┬─────────────────────────┘  │
│                    │ IPC                        │
│  ┌─────────────────┴─────────────────────────┐  │
│  │           Rust Backend                    │  │
│  │                                           │  │
│  │  ┌─────────────┐    ┌─────────────────┐   │  │
│  │  │ Project     │    │ MCP Server      │   │  │
│  │  │ Store       │    │ (localhost)      │   │  │
│  │  │             │    │                 │   │  │
│  │  │ SQLite      │◄──►│ Tools for:      │   │  │
│  │  │ (per project│    │ items, notes,   │   │  │
│  │  │  .lsvr file)│    │ ioi, connections│   │  │
│  │  │             │    │ tags, search    │   │  │
│  │  └─────────────┘    └─────────────────┘   │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
         ▲                        ▲
         │ UI interaction         │ MCP (streamable-HTTP on 127.0.0.1)
         │                        │
    Researcher              Claude Code / Codex
                            (with pyghidra-mcp
                             for Ghidra access)
```

## Components

> **Human/agent parity.** The frontend is a full read-**write** client: every
> mutating MCP tool has a human CRUD affordance (`human >= agent`), enforced in
> CI. See [ui.md](ui.md#humanagent-parity).

- **Dashboard**: Project overview — item list, severity/triage breakdown, recent findings (home view).
- **Tab Bar**: Open items as tabs (open/close/switch). Each tab is an item.
- **Item Detail View**: An item's notes, items of interest, and connections, with create/edit/delete affordances.
- **Connection Map**: Cytoscape.js graph showing items and their connections across the project; click a node to open it as a tab.
- **Sidebar**: Navigation — all items / by severity / explanations / managers.
- **Tag & Connection-Type Managers**: CRUD for the registered vocabularies.
- **Search/Filter view**: Parity with the `search` / `filter` tools.
- **Status Bar**: MCP server port and project counts.
- **Zustand Store**: Client-side view state. Holds the latest project snapshot.

### Rust Backend

- **Project Store**: CRUD + delete for all entities. SQLite database (one `.lsvr` file per project) with FTS5 for full-text search.
- **MCP Server**: streamable-HTTP server on `127.0.0.1` (the `/mcp` endpoint; also supports a stdio transport in the headless binary). Starts automatically when a project is opened. Agents connect over HTTP.

The Rust crate exposes `db` and `mcp` as pure modules — nothing in them depends
on Tauri — so both the desktop app and the standalone headless binary
(see below) link against the same code path.

## Headless MCP binary

A second binary target, `liteskillvr-mcp`, runs just the `db` + `mcp` layer
with no GUI. It supports two transports:

- **HTTP** (default) — same endpoint as the desktop app (`/mcp`), on any port
- **stdio** — JSON-RPC on stdin/stdout, for MCP clients that spawn the server
  as a subprocess

```
liteskillvr-mcp path/to/project.lsvr                 # HTTP on 27182
liteskillvr-mcp --port 3000 path/to/project.lsvr     # HTTP on custom port
liteskillvr-mcp --stdio path/to/project.lsvr         # stdio transport
liteskillvr-mcp --init path/to/project.lsvr          # create if missing
```

Built with Cargo features: `gui` (default, enables Tauri) gates the desktop
binary; building with `--no-default-features` produces a truly headless binary
with no WebKitGTK/Tauri link-time dependencies. Pre-built binaries for Linux,
macOS (x86_64 + aarch64), and Windows ship with each GitHub release. The Linux
`.deb` / `.rpm` also install `liteskillvr-mcp` to `/usr/bin/` alongside the
desktop app.

## Data Flow

1. User opens LiteSkill VR → it opens or creates `project.lsvr` in the working directory → the MCP server starts automatically.
2. The frontend calls `project_snapshot` once and renders the whole project.
3. User starts Claude Code (or Codex) pointed at the MCP server (port shown in the status bar).
4. The agent calls `project_summary` to orient itself, then reads/writes entities via MCP tools — **all writes go through MCP**.
5. Every backend mutation emits a `db-changed` event; the frontend re-fetches `project_snapshot` and re-renders. The UI is always a complete, consistent view of the database.
6. User navigates the UI to review findings (read-only today).

## IPC Contract

The Tauri IPC surface is deliberately tiny: **one read command, one write
command, one event**.

```
invoke("project_snapshot") → {            // READ
  items,             // Item summaries
  details,           // full ItemDetail per item (notes + ioi + connections)
  tags,              // registered Tag[]
  connection_types,  // registered ConnectionType[]
  explanations, explanation_details,
  mcp_port           // where agents connect (e.g. 27182)
}

invoke("mcp_call", { tool, args }) → result   // WRITE (and any tool call)
  // Runs the SAME handlers::dispatch the MCP server uses, stamped
  // author_type = "human" with the OS username. This is what guarantees
  // human/agent parity — UI and agent writes share one code path.

listen("db-changed", callback)  // emitted on every backend mutation; the
                                // frontend responds by re-fetching the snapshot
```

There is a single write path (`dispatch`) and a single, coarse sync signal
(`db-changed`) — important because an AI agent mutates the database concurrently
with the user, and because it makes UI and agent behaviour provably identical.

## Persistence

Each project is a single `.lsvr` file (SQLite, WAL mode, foreign keys on).
Projects can be backed up or shared by copying the file.

> **Implementation status.** The desktop app currently opens (or creates)
> `project.lsvr` in the current working directory; there is no open/new file
> dialog or OS file-association yet. Those are planned — see
> [file-formats.md](file-formats.md).
