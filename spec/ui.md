# UI Specification

## Design Philosophy

The UI is built for **rapid navigation across a large attack surface** — multiple binaries, shared objects, and firmware components. The researcher needs to hop between functions, binaries, findings, and graphs without losing context. Think less "IDE with panels" and more "browser with tabs, back/forward, and instant search."

Key principles:

- **Speed over structure**: every view is reachable in 1-2 keystrokes
- **Multi-binary aware**: the project may contain dozens of `.so` files and binaries from a firmware image; the UI treats this as a navigable tree, not a flat list
- **Context trails**: the app remembers where you've been and lets you retrace your steps
- **Minimal chrome**: maximize canvas/content area, collapse everything else on demand

## Layout

Single full-bleed viewport with floating/overlay elements. No permanent sidebar or fixed panels.

```
┌──────────────────────────────────────────────────────────┐
│ ◄ ►  Breadcrumb: firmware > libfoo.so > parse_header    │
│      ┌──────────────────────────────────────────┐        │
│      │                                          │        │
│      │                                          │        │
│      │          Active View (full bleed)        │        │
│      │                                          │        │
│      │    Call Graph / Disasm / Finding /        │        │
│      │    Cross-ref Map / Diff                  │        │
│      │                                          │        │
│      │                                          │        │
│      │                                          │        │
│      └──────────────────────────────────────────┘        │
│                                                          │
│  ┌─────────┐                            ┌─────────────┐ │
│  │ Pinned  │                            │ Chat (⌘/)   │ │
│  │ Notes   │                            │ collapsed   │ │
│  └─────────┘                            └─────────────┘ │
├──────────────────────────────────────────────────────────┤
│  firmware.bin  ▸ libfoo.so  ▸ libbar.so  ▸ httpd  [+]  │
└──────────────────────────────────────────────────────────┘
```

### Navigation Bar (top)

- **Back / Forward buttons**: navigate your history stack (like a browser)
- **Breadcrumb**: shows current location in the hierarchy — clickable at every level
  - Firmware → Binary/SO → Function → Basic Block (optional depth)
- **Ctrl+K / ⌘K**: command palette — fuzzy search across all functions, binaries, findings, annotations in the entire project
- Breadcrumb segments are clickable and have dropdown menus showing siblings (other functions in the same binary, other binaries in the firmware)

### Binary Tab Bar (bottom)

- Horizontal tabs for each open binary / `.so` — like browser tabs
- Tabs show the binary name and an indicator for number of findings
- Right-click tab: close, close others, show in firmware tree
- Drag to reorder
- `[+]` button or Ctrl+O to open another binary from the firmware
- Tabs persist across sessions

### Active View (center)

Full-bleed content area. The active view is one of:

| View                    | Purpose                                                                 | Enter via                                  |
| ----------------------- | ----------------------------------------------------------------------- | ------------------------------------------ |
| **Call Graph**          | Interactive graph for current binary or cross-binary                    | Click function, Ctrl+G                     |
| **Function Detail**     | Decompiled/disasm view + annotations for a single function              | Double-click graph node, search            |
| **Finding Editor**      | Document a vulnerability                                                | Ctrl+N, or from context menu               |
| **Cross-Reference Map** | Where is this function/symbol used across all binaries?                 | Right-click → "Find xrefs across firmware" |
| **Firmware Overview**   | Grid/tree of all binaries, their shared symbols, attack surface summary | Home key or breadcrumb root click          |
| **Diff View**           | Compare two functions or two versions of a binary                       | From command palette                       |

Views **stack** — opening a function detail from a call graph pushes onto the history. Back button returns to the graph at the same scroll/zoom position.

### Floating Elements

These appear on demand and can be dismissed or pinned:

- **Chat overlay** (⌘/ or Ctrl+/): slides in from the right edge, 40% width. Can be pinned to stay open. Transparent to the view underneath when unpinned.
- **Pinned notes**: small floating card (bottom-left) showing your scratchpad for the current research session. Stays visible across view changes.
- **Quick annotations**: press `a` on any selected node/function to pop a small inline annotation editor — no context switch needed.
- **Minimap**: optional floating minimap for large call graphs (toggle with `m`)

## Navigation Model

### History Stack

Every navigation action pushes to a history stack:

```
firmware overview
  → opened libfoo.so call graph
    → clicked parse_header node → function detail
      → followed xref to libbar.so:validate_input
        → back → back → back to firmware overview
```

Ctrl+[ / Ctrl+] (or mouse back/forward) traverse the stack. The stack preserves view state (scroll position, zoom level, selected nodes).

### Bookmarks

- Press `b` to bookmark the current location (binary + function + view)
- Ctrl+B opens bookmark list
- Bookmarks can be named and grouped
- "Interesting functions" is a natural bookmark group that feeds into AI context

### Jump-to

- `g` then type: jump to any function by name across all binaries
- `G` then type: jump to any address
- `f` then type: jump to any finding
- These are instant — no modal dialogs, just a floating input that filters as you type

## Firmware-Aware Features

### Firmware Tree

The project models a firmware image as a hierarchy:

```
Firmware Image
├── filesystem/
│   ├── usr/lib/
│   │   ├── libcrypto.so
│   │   ├── libfoo.so
│   │   └── libbar.so
│   ├── usr/bin/
│   │   ├── httpd
│   │   └── cli_manager
│   └── etc/
│       └── config.xml
├── kernel modules/
│   └── custom_driver.ko
└── bootloader/
    └── u-boot.bin
```

- Accessible from the firmware overview or breadcrumb root
- Shows shared symbol dependencies between binaries (which `.so` exports are consumed by which binaries)
- Color-coded by analysis status: untouched, in-progress, reviewed

### Cross-Binary Analysis

Core feature for firmware RE:

- **Shared symbol map**: which binaries import/export the same symbols
- **Cross-binary call graph**: trace a call path that crosses `.so` boundaries (e.g., `httpd` → `libfoo.so:parse_header` → `libcrypto.so:EVP_DecryptUpdate`)
- **Cross-binary xrefs**: "where is this function used across the entire firmware?"
- **Attack surface view**: list all exported functions across all binaries, sorted by reachability from external inputs (network, USB, serial, etc.)

### Binary Comparison

- Diff two versions of the same binary (firmware update analysis)
- Function-level diffing: matched by name/signature, shows added/removed/changed functions
- Useful for 1-day analysis and patch diffing

## Interaction Patterns

### Keyboard-First

Every action has a keyboard shortcut. The UI is usable without a mouse.

| Key                 | Action                                       |
| ------------------- | -------------------------------------------- |
| `⌘K` / `Ctrl+K`     | Command palette (fuzzy search everything)    |
| `g`                 | Go to function (by name)                     |
| `G`                 | Go to address                                |
| `f`                 | Go to finding                                |
| `b`                 | Bookmark current location                    |
| `Ctrl+B`            | Open bookmarks                               |
| `a`                 | Annotate selected item                       |
| `n`                 | New finding from current context             |
| `⌘/` / `Ctrl+/`     | Toggle chat overlay                          |
| `m`                 | Toggle minimap                               |
| `Ctrl+[` / `Ctrl+]` | History back / forward                       |
| `Ctrl+G`            | Open call graph for current binary           |
| `x`                 | Show cross-references for selected function  |
| `Tab`               | Cycle between open binary tabs               |
| `Shift+Tab`         | Cycle backward                               |
| `Esc`               | Dismiss overlay / deselect / go up one level |
| `?`                 | Show keyboard shortcut cheatsheet            |

### Context Menus

Right-click on any entity for contextual actions:

- **Function node**: annotate, mark source/sink, trace paths, find xrefs, ask agent, create finding
- **Binary tab**: close, show in firmware tree, view exports, compare with...
- **Finding**: change severity/status, link to function, ask agent to elaborate

### Drag Interactions

- Drag a function node onto a finding to link them
- Drag a binary from firmware tree onto the tab bar to open it
- Drag to rearrange tabs

## Chat Integration

The chat overlay is contextual — it knows where you are:

- If you're viewing a function: agent sees the function's decompilation, annotations, and position in the call graph
- If you're on a cross-binary xref view: agent sees the full cross-reference chain
- If you're on firmware overview: agent sees the attack surface summary

The chat is a **tool for the current moment**, not a separate workspace. Ask "is this function reachable from the network handler?" and the agent answers with references you can click to navigate.

## Theming

- Dark mode default
- Light mode available
- Syntax highlighting for disassembly and decompiled code (Shiki)
- Graph node colors consistent across views (red=sink, green=source, yellow=path, blue=annotated)
