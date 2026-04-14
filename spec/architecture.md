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
         │ UI interaction         │ MCP (stdio)
         │                        │
    Researcher              Claude Code / Codex
                            (with pyghidra-mcp
                             for Ghidra access)
```

## Components

### Frontend (TypeScript / React)

- **Tab Bar**: Open items as tabs, browser-like navigation
- **Item Browser**: List of all items in the project with status and summary stats
- **Item Detail View**: Notes, items of interest, connections for the selected item
- **Connection Map**: Cytoscape.js graph showing items and their connections across the project
- **Search**: Full-text search with results across all entity types
- **Tag Manager**: View, create, and edit registered tags and connection types
- **Zustand Store**: Client-side state, syncs with Rust backend via Tauri IPC

### Rust Backend

- **Project Store**: CRUD + delete for all entities. SQLite database (one `.lsvr` file per project) with FTS5 for full-text search.
- **MCP Server**: Hosts MCP tools on localhost (stdio transport). Starts automatically with the app.

## Data Flow

1. User opens LiteSkill VR → opens or creates a `.lsvr` project file → MCP server starts.
2. User creates items and adds notes via the UI.
3. User starts Claude Code with `liteskill-mcp` configured.
4. Claude calls `project_summary` to orient itself, then reads/writes entities via MCP tools.
5. Frontend receives updates via IPC events and renders them in real time.
6. User navigates the UI to review, edit, search, delete, and annotate.

## IPC Contract

Frontend ↔ Backend communication uses Tauri `invoke` (request/response) and `listen`/`emit` (events).

```
invoke("project_get") → Project
invoke("item_list", { filters }) → Item[]
invoke("item_get", { id }) → ItemDetail (item + notes + ioi + connections)
invoke("item_create", { data }) → Item
invoke("item_update", { id, data }) → Item
invoke("item_delete", { id }) → void
invoke("note_create", { item_id, data }) → Note
invoke("note_delete", { id }) → void
invoke("ioi_create", { item_id, data }) → ItemOfInterest
invoke("ioi_delete", { id }) → void
invoke("connection_create", { data }) → Connection
invoke("connection_delete", { id }) → void
invoke("connection_list", { entity_id }) → Connection[] (both directions)
invoke("connection_list_all") → Connection[] (project-wide)
invoke("tag_list") → Tag[]
invoke("tag_create", { data }) → Tag
invoke("tag_delete", { id }) → void
invoke("connection_type_list") → ConnectionType[]
invoke("connection_type_create", { data }) → ConnectionType
invoke("search", { query, filters }) → SearchResult[]
invoke("changes_since", { timestamp }) → ChangeSet

listen("entity_changed", callback) // fired when MCP or UI mutates data
```

## Persistence

Each project is a single `.lsvr` file (SQLite with custom extension). The app opens project files via a standard file dialog. Projects can be backed up or shared by copying the file.
