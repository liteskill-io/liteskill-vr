# MCP Server

## Overview

LiteSkill VR hosts an MCP (Model Context Protocol) server that starts automatically when the app opens. AI agents like Claude Code or Codex connect to it and get tools for reading and writing research data.

The MCP server is a **read-write interface**. Agents don't just dump findings — they query existing research, review what past sessions discovered, build on prior analysis, and avoid duplicating work.

## Connection

Configuration for Claude Code's MCP settings:

```json
{
  "mcpServers": {
    "liteskill": {
      "command": "liteskill-vr",
      "args": ["--mcp"]
    }
  }
}
```

When invoked with `--mcp`, the app starts the MCP server on stdio without opening a window. The full UI can be opened separately and will connect to the same database.

### Author Identity

The agent's identity is established at connection time via an environment variable or CLI argument:

```json
{
  "mcpServers": {
    "liteskill": {
      "command": "liteskill-vr",
      "args": ["--mcp", "--author", "claude-code"]
    }
  }
}
```

All entities created through this connection are automatically stamped with `author: "claude-code"` and `author_type: "agent"`. The author is never passed per-call — it's a property of the connection. This prevents drift ("claude" vs "Claude" vs "claude-code").

Human-created entities via the UI use `author_type: "human"` with the OS username as the author.

## Design Principles

1. **Read before write**: Check what exists before creating. `project_summary` and `filter` orient the agent.
2. **Batch writes**: All create operations have batch variants. Batches are all-or-nothing — if any entry fails validation, the entire batch is rejected with the specific error. No partial creates.
3. **Duplicate detection**: `ioi_create` warns when a similar entity already exists rather than silently duplicating.
4. **Registered vocabularies**: Tags and connection types must be registered before use, preventing drift across sessions.
5. **Two query modes**: `search` for full-text queries, `filter` for structured queries without text. Both are needed.
6. **Session continuity**: `changes_since` shows what happened since the agent's last session.
7. **Mistake recovery**: `bulk_delete` undoes an entire bad session in one call.

## Tools (26 total)

### Project

| Tool              | Params             | Returns                                                                                                                                             |
| ----------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `project_get`     | —                  | Project metadata                                                                                                                                    |
| `project_summary` | —                  | All items with status/counts, severity breakdown, recent activity, registered tags, registered connection types. Orients an agent at session start. |
| `changes_since`   | `since: timestamp` | All entities created or updated after the timestamp, grouped by type.                                                                               |

### Tags

| Tool         | Params                          | Returns                                    |
| ------------ | ------------------------------- | ------------------------------------------ |
| `tag_list`   | —                               | `Tag[]`. **Call before tagging anything.** |
| `tag_create` | `name`, `description`, `color?` | `Tag`. Fails if name exists.               |
| `tag_delete` | `id`                            | void. Removes from all entities.           |

### Connection Types

| Tool                     | Params                | Returns                                                   |
| ------------------------ | --------------------- | --------------------------------------------------------- |
| `connection_type_list`   | —                     | `ConnectionType[]`. **Call before creating connections.** |
| `connection_type_create` | `name`, `description` | `ConnectionType`. Fails if name exists.                   |

Default connection types: `calls`, `imports`, `links`, `reads_config`, `writes_config`, `spawns`, `related`.

### Items

| Tool                | Params                                                                      | Returns                                                                                       |
| ------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `item_list`         | `item_type?`, `analysis_status?`, `tags?`                                   | `Item[]` with note/ioi/connection counts per item.                                            |
| `item_get`          | `id`                                                                        | `Item` + all `Note[]` + `ItemOfInterest[]` + `Connection[]`. Always returns the full context. |
| `item_create`       | `name`, `item_type`, `path?`, `architecture?`, `description`, `tags?`       | `Item`. Tags must be registered.                                                              |
| `item_create_batch` | `items: Array<{name, item_type, path?, architecture?, description, tags?}>` | `Item[]`. All-or-nothing.                                                                     |
| `item_update`       | `id`, mutable fields                                                        | `Item`                                                                                        |
| `item_delete`       | `id`                                                                        | void. Cascades to notes, ioi, connections.                                                    |

### Notes

| Tool                | Params                                           | Returns                                                               |
| ------------------- | ------------------------------------------------ | --------------------------------------------------------------------- |
| `note_create`       | `item_id`, `title`, `content`, `tags?`           | `Note`                                                                |
| `note_create_batch` | `notes: Array<{item_id, title, content, tags?}>` | `Note[]`. All-or-nothing. Notes can span multiple items in one batch. |
| `note_update`       | `id`, mutable fields                             | `Note`                                                                |
| `note_delete`       | `id`                                             | void                                                                  |

### Items of Interest

| Tool               | Params                                                                       | Returns                                                                                              |
| ------------------ | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `ioi_create`       | `item_id`, `title`, `description`, `location?`, `severity?`, `tags?`         | `ItemOfInterest`. Includes `duplicate_warning` if similar title or location exists on the same item. |
| `ioi_create_batch` | `item_id`, `items: Array<{title, description, location?, severity?, tags?}>` | `ItemOfInterest[]`. All-or-nothing. Each entry includes `duplicate_warning` if applicable.           |
| `ioi_update`       | `id`, mutable fields                                                         | `ItemOfInterest`                                                                                     |
| `ioi_delete`       | `id`                                                                         | void. Cascades to connections.                                                                       |

### Connections

| Tool                      | Params                                                                                               | Returns                                              |
| ------------------------- | ---------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `connection_create`       | `source_id`, `source_type`, `target_id`, `target_type`, `connection_type`, `description`             | `Connection`. `connection_type` must be registered.  |
| `connection_create_batch` | `connections: Array<{source_id, source_type, target_id, target_type, connection_type, description}>` | `Connection[]`. All-or-nothing.                      |
| `connection_list`         | `entity_id`, `connection_type?`                                                                      | `Connection[]` where entity is source **or** target. |
| `connection_list_all`     | —                                                                                                    | All connections in the project.                      |
| `connection_delete`       | `id`                                                                                                 | void                                                 |

### Search & Filter

| Tool     | Params                                                                                                             | Returns                                                                                                                          |
| -------- | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `search` | `query` (required), `entity_type?`, `tags?`, `severity?`, `connection_type?`, `author_type?`                       | Full-text search via FTS5. Returns matches with highlighted snippets and parent context. All filters optional and combinable.    |
| `filter` | `entity_type` (required), `tags?`, `severity?`, `connection_type?`, `author_type?`, `item_id?`, `analysis_status?` | Structured query with no text search. Returns matching entities. Use for "all critical IOIs" or "all connections of type calls." |

### Bulk Operations

| Tool          | Params                              | Returns                                                                                                                                                                                       |
| ------------- | ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bulk_delete` | `author?`, `since?`, `entity_type?` | Deletes all matching entities. At least one filter required. Returns count of deleted entities. For undoing a bad agent session: `bulk_delete(author="claude-code", since="2024-01-15T...")`. |

## Typical Agent Session

```
1. Orient
   project_summary()                → current state of all research
   tag_list()                       → available tags
   connection_type_list()           → available connection types

2. Catch up (if not first session)
   changes_since("2024-01-15T...")  → what's new since last time

3. Review a target
   item_get(id)                     → everything known about this item

4. Analyze (via external tools)
   [pyghidra-mcp calls]            → decompile, get xrefs, etc.

5. Document findings (batch)
   ioi_create_batch(item_id, [
     { title: "parse_header()", severity: "high", ... },
     { title: "auth_check()",   severity: "critical", ... },
   ])
   note_create(item_id, ...)       → summary note

6. Draw connections (batch)
   connection_create_batch([
     { source: httpd, target: libfoo, type: "links", ... },
     { source: httpd, target: httpd_conf, type: "reads_config", ... },
   ])

7. Structured queries
   filter(entity_type="item_of_interest", severity="critical")
                                    → all critical findings project-wide
   filter(entity_type="connection", connection_type="calls")
                                    → all call relationships

8. Correct mistakes
   ioi_delete(id)                   → remove a false positive
   bulk_delete(author="claude-code", since="...", entity_type="item_of_interest")
                                    → undo an entire bad batch
```

## Batch Semantics

All `_batch` operations are **transactional**. If any entry in the batch fails validation (invalid tag, unregistered connection type, missing required field), the entire batch is rejected. The error response identifies which entry failed and why. No partial creates occur.

Example error:

```json
{
  "error": "batch_validation_failed",
  "index": 3,
  "message": "Tag 'buffer-overflow' is not registered. Did you mean 'memory-corruption'?",
  "suggestion": "Call tag_list() to see registered tags, or tag_create() to register a new one."
}
```

## Security

- MCP server binds to `127.0.0.1` only
- All mutations stamped with the connection's author identity and timestamp
- `bulk_delete` requires at least one filter to prevent accidental full wipe
