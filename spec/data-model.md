# Data Model

Each project is a standalone SQLite database file (`.lsvr`). All entities live within that database. No actual files are stored — items are documentation entries representing targets of interest.

## Author Identity

Every entity that tracks authorship (`Note`, `ItemOfInterest`, `Connection`) has `author` and `author_type` fields. These are **not** set per-call:

- **MCP connections**: Author is established at connection time via the `--author` CLI argument. All entities created through that connection are automatically stamped. `author_type` is always `"agent"`.
- **UI**: Author is the OS username. `author_type` is always `"human"`.

This prevents drift across calls ("claude" vs "Claude" vs "claude-code").

## Core Entities

### Project

Metadata stored in the database itself. One database = one project.

```typescript
interface Project {
  id: string;
  name: string;
  description: string;
  created_at: string; // ISO 8601
  updated_at: string;
}
```

### Tag

Tags must be registered before use. A starter set is provided on project creation.

```typescript
interface Tag {
  id: string;
  name: string; // unique within project
  description: string;
  color?: string; // hex color for UI
  created_at: string;
}
```

Default tags on project creation:

- `memory-corruption` — Buffer overflows, heap issues, use-after-free
- `auth-bypass` — Authentication or authorization flaws
- `command-injection` — OS command injection
- `hardcoded-creds` — Hardcoded passwords, keys, tokens
- `info-disclosure` — Information leakage
- `logic-issue` — Business logic or control flow flaws
- `crypto-weakness` — Weak or misused cryptography
- `race-condition` — TOCTOU and concurrency bugs
- `format-string` — Format string vulnerabilities
- `integer-issue` — Integer overflow, underflow, truncation
- `insecure-config` — Dangerous default or misconfiguration
- `debug-interface` — Debug ports, test endpoints, JTAG
- `interesting` — Worth investigating further (not yet classified)

### ConnectionType

Connection types must be registered before use, preventing drift across sessions.

```typescript
interface ConnectionType {
  id: string;
  name: string; // unique within project
  description: string;
  created_at: string;
}
```

Default connection types on project creation:

- `calls` — Source function/binary calls target function/binary
- `imports` — Source imports a symbol from target
- `links` — Source dynamically links target shared object
- `reads_config` — Source reads target config file at runtime
- `writes_config` — Source writes/modifies target config file
- `spawns` — Source starts target as a process/daemon
- `related` — Loose association worth tracking

### Item

A documentation entry representing any target of interest. No actual files are stored.

```typescript
interface Item {
  id: string;
  name: string;
  item_type: string; // freeform: "elf", "shared_object", "kernel_module", "script", "config", etc.
  path?: string; // original path (for reference only)
  architecture?: string; // e.g. "arm32", "mips-le", "x86_64"
  description: string; // markdown
  analysis_status: "untouched" | "in_progress" | "reviewed";
  tags: string[]; // must reference registered Tag names
  created_at: string;
  updated_at: string;
}
```

### Note

Freeform markdown notes attached to an item.

```typescript
interface Note {
  id: string;
  item_id: string;
  title: string;
  content: string; // markdown
  author: string; // set automatically from connection identity
  author_type: "human" | "agent";
  tags: string[]; // must reference registered Tag names
  created_at: string;
  updated_at: string;
}
```

### ItemOfInterest

Anything notable about an item. Intentionally untyped — a function, a string, a config line, a suspicious pattern, a potential bug.

```typescript
interface ItemOfInterest {
  id: string;
  item_id: string;
  title: string;
  description: string; // markdown
  location?: string; // freeform: address, line number, offset, symbol name
  severity?: "critical" | "high" | "medium" | "low" | "info";
  author: string; // set automatically from connection identity
  author_type: "human" | "agent";
  tags: string[]; // must reference registered Tag names
  created_at: string;
  updated_at: string;
}
```

### Connection

A relationship between any two entities (item-item, item-ioi, ioi-ioi). Bidirectional for queries.

```typescript
interface Connection {
  id: string;
  source_id: string;
  source_type: "item" | "item_of_interest";
  target_id: string;
  target_type: "item" | "item_of_interest";
  connection_type: string; // must reference registered ConnectionType name
  description: string; // markdown
  author: string; // set automatically from connection identity
  author_type: "human" | "agent";
  created_at: string;
}
```

Connection examples:

| Source                   | Target                         | Type         | Description                               |
| ------------------------ | ------------------------------ | ------------ | ----------------------------------------- |
| httpd (item)             | libfoo.so (item)               | links        | httpd dynamically links libfoo.so         |
| httpd:cmd_handler (ioi)  | libfoo.so:validate_input (ioi) | calls        | cmd_handler calls validate_input for auth |
| httpd (item)             | /etc/httpd.conf (item)         | reads_config | httpd reads this config at startup        |
| init.sh (item)           | httpd (item)                   | spawns       | init script starts httpd as a daemon      |
| httpd:parse_header (ioi) | httpd:auth_check (ioi)         | related      | both share unsanitized user input         |

## Deletion

All entities support deletion:

- Deleting an **item** cascades to its notes, items of interest, and any connections referencing it.
- Deleting an **item of interest** cascades to its connections.
- Deleting a **tag** removes it from all entities that use it.
- Deleting a **connection type** removes all connections of that type.
- **Bulk delete** filters by `author`, `since` (timestamp), and `entity_type`. At least one filter is required. For undoing a bad agent session.

## Duplicate Detection

When creating an item of interest, the system checks for existing entries on the same item with a similar title or matching location. If a potential duplicate is found, the response includes a `duplicate_warning` field with the ID and title of the existing entry. The create still succeeds — the agent or user decides whether to proceed.

## Search & Filter

Two query modes:

**`search`** — Full-text search via SQLite FTS5. Requires a text query. Returns matches with highlighted snippets and parent context. Optional filters narrow results: `entity_type`, `tags`, `severity`, `connection_type`, `author_type`.

**`filter`** — Structured query with no text search. Requires `entity_type`. Returns all entities matching the filter params: `tags`, `severity`, `connection_type`, `author_type`, `item_id`, `analysis_status`. Use for queries like "all critical IOIs" or "all connections of type calls."

## Batch Semantics

All `_batch` create operations are transactional. If any entry fails validation (invalid tag, unregistered connection type, missing required field), the entire batch is rejected. The error identifies which entry failed, why, and suggests a fix. No partial creates occur.

## Storage

Each project is a single `.lsvr` file (SQLite with custom extension). Projects can be backed up, shared, or archived by copying the file. The app opens/creates project files via a standard file dialog.
