# UI Specification

> **Core requirement: human/agent parity.** There must be **zero things an AI
> agent can do that a human cannot do in the UI.** Every mutating MCP tool has a
> corresponding human CRUD affordance (`human >= agent`), enforced in CI (see
> [Parity](#humanagent-parity)). The UI is therefore a full read-**write**
> client, not a viewer.
>
> Reads come from the `project_snapshot` IPC command; writes go through a single
> `mcp_call(tool, args)` IPC command that runs the **same dispatch** as the MCP
> server, stamped `author_type: "human"`. After any write the UI refetches the
> snapshot on `db-changed`. Some niceties below remain **Planned** (tagged):
> the command palette, markdown rendering + syntax highlighting, tab
> badges/reordering, and breadcrumb/back-forward history.

## Human/agent parity

Everything an agent can change, a human can change. The MCP `MUTATION_TOOLS`
registry is the source of truth; `src/lib/capabilities.ts` maps each tool to the
UI control that exposes it; `scripts/check-parity.mjs` (`task parity:check`,
wired into `task check` and CI) fails the build if any mutating tool lacks a UI
affordance. The `*_batch` tools are allowlisted — a human creating entries one
at a time reaches the same state, so the capability is covered.

Writes are modal/drawer forms (create + edit) with confirm-on-delete; the
destructive `bulk_delete` sits behind an explicit danger confirm. Forms prefill
registered tags and connection types from the snapshot so a human can't trip the
"unregistered vocabulary" validation, and surface any dispatch error inline.

## Design Philosophy

The UI is built for rapid navigation across a large project. The researcher needs to hop between items, search for patterns, review AI-generated findings, and correct mistakes without losing context.

Key principles:

- **Tabs, not panels**: each item opens in a tab, like a browser
- **Speed over structure**: every view reachable in 1-2 keystrokes
- **Search everything**: full-text search across all notes, items of interest, and connections
- **Minimal chrome**: maximize content area, collapse everything else on demand

## Layout

```
┌──────────────────────────────────────────────────────────┐
│ ◄ ►  Breadcrumb: project > httpd > parse_header       │
├──────────────────────────────────────────────────────────┤
│                                                          │
│                  Active Tab (full bleed)                  │
│                                                          │
│   Item Detail / Connection Map / Search Results          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
├──────────────────────────────────────────────────────────┤
│  httpd  ▸ libfoo.so  ▸ httpd.conf  ▸ init.sh  [+]      │
├──────────────────────────────────────────────────────────┤
│  MCP: listening  │  12 items  │  3 critical  │  5 high  │
└──────────────────────────────────────────────────────────┘
```

### Navigation Bar (top) — Planned

Not built. Navigation today is via the Sidebar (all items / by severity) and the
Tab Bar. The intended design:

- **Back / Forward**: browser-like history stack
- **Breadcrumb**: project > item > item of interest (clickable at every level)
- **Ctrl+K / ⌘K**: command palette — fuzzy search across all entities

### Tab Bar (bottom of content area)

- One tab per open item, showing the item name; click to switch, click ✕ to close (**Built**)
- **Planned:** a badge for the number of items of interest, right-click close/close-others, drag-to-reorder, and `[+]`/`Ctrl+O` to open another item

### Status Bar

- MCP server status
- Project stats (item count, severity breakdown)

## Views

### Project Overview

Home view when no item tab is focused.

- List/grid of all items in the project
- Grouped by item_type or flat list (toggle)
- Color-coded by analysis_status (untouched, in_progress, reviewed)
- Quick stats per item (note count, ioi count, connection count)
- Tag filter sidebar

### Item Detail

Main view when an item tab is active. Each section has create/edit/delete
affordances (modal forms; delete confirms). Three sections:

**Header**: Item name, type, path, architecture, status, tags. Editable inline. Delete button.

**Items of Interest** (main area):

- List of all items of interest for this item
- Each shows title, severity badge, location, description preview, tags
- Click to expand full description
- Inline edit and delete
- "Add" button or `n` key to create new

**Notes** (collapsible section):

- Markdown-rendered notes
- Each note shows author (human/agent badge), timestamp, tags
- Inline edit and delete
- "Add" button or `a` key to create new

**Connections** (collapsible section):

- List of connections from/to this item or its items of interest
- Each shows: source → target, connection type, description
- Click a connection to navigate to the other end
- Delete button on each connection
- "Connect" button to draw a new connection

### Connection Map

Project-wide view showing all items and their connections as a graph.

- Items are nodes, connections are edges
- Edge labels show connection type
- Nodes colored by analysis_status
- Node size scaled by ioi count
- Click an item node to open it as a tab
- Useful for understanding how components relate across a firmware image

### Tag Manager

Full CRUD for the registered tag vocabulary (parity with `tag_create` /
`tag_delete`), plus the connection-type vocabulary (Connection-Type Manager).
Accessible from the sidebar.

- List of all registered tags with name, description, color, and usage count
- Create new tags
- Edit tag descriptions and colors
- Delete tags (removes from all entities)

### Search Results

A search/filter view giving humans parity with the `search` and `filter` tools.

Full-screen search results view.

- Results grouped by entity type (items, notes, items of interest, connections)
- Each result shows a snippet with the match highlighted
- Click to navigate to the result in context
- Filter by entity type, severity, tags

## Keyboard Shortcuts

> **Planned**, except the zoom shortcuts. Today only `Ctrl`/`⌘` `+` / `-` / `0`
> (zoom in / out / reset) are wired up. The rest of this table is a design
> target — none of `⌘K`, `n`, `a`, `c`, `Tab`, `Esc`, `?`, or `Delete` is
> implemented yet.

| Key                    | Action                                     |
| ---------------------- | ------------------------------------------ |
| `⌘K` / `Ctrl+K`        | Command palette (search everything)        |
| `n`                    | New item of interest on current item       |
| `a`                    | New note on current item                   |
| `c`                    | New connection from current context        |
| `Tab`                  | Cycle between open tabs                    |
| `Shift+Tab`            | Cycle backward                             |
| `Ctrl+[` / `Ctrl+]`    | History back / forward                     |
| `Esc`                  | Dismiss overlay / go up one level          |
| `?`                    | Show keyboard shortcut cheatsheet          |
| `Delete` / `Backspace` | Delete selected entity (with confirmation) |

## Real-Time Updates

When an AI agent creates, updates, or deletes entities via MCP, the UI updates immediately via Tauri IPC events.

## Theming

- Dark mode (the only theme today; a monospace, terminal-leaning aesthetic).
- **Planned:** a light mode, markdown rendering of notes/descriptions (they are
  currently shown as escaped plain text), and syntax highlighting for code
  snippets. No markdown or highlighting library is wired up yet.
