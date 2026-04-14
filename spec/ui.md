# UI Specification

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

### Navigation Bar (top)

- **Back / Forward**: browser-like history stack
- **Breadcrumb**: project > item > item of interest (clickable at every level)
- **Ctrl+K / ⌘K**: command palette — fuzzy search across all entities

### Tab Bar (bottom of content area)

- One tab per open item
- Tabs show item name and a badge for number of items of interest
- Right-click: close, close others
- Drag to reorder
- `[+]` or Ctrl+O to open another item

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

Main view when an item tab is active. Three sections:

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

Accessible from project settings or command palette.

- List of all registered tags with name, description, color, and usage count
- Create new tags
- Edit tag descriptions and colors
- Delete tags (removes from all entities)

### Search Results

Full-screen search results view.

- Results grouped by entity type (items, notes, items of interest, connections)
- Each result shows a snippet with the match highlighted
- Click to navigate to the result in context
- Filter by entity type, severity, tags

## Keyboard Shortcuts

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

- Dark mode default
- Light mode available
- Syntax highlighting for code snippets in notes (Shiki)
