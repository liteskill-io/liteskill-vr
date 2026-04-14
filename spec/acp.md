# ACP Integration

## Overview

LiteSkill VR uses the [Agent Communication Protocol (ACP)](https://github.com/anthropics/acp) to connect with AI agents such as Claude Code and Codex. ACP provides a standardized way for the application to expose project context and tools to agents, and for agents to stream responses and invoke actions.

## Architecture

```
┌──────────────┐       ACP (HTTP/SSE)       ┌──────────────┐
│  LiteSkill   │◄──────────────────────────►│  AI Agent    │
│  VR          │                            │  (Claude,    │
│              │  - context documents       │   Codex,     │
│  ACP Server  │  - tool definitions        │   custom)    │
│  (Rust)      │  - tool call results       │              │
│              │  - streaming responses     │              │
└──────────────┘                            └──────────────┘
```

LiteSkill VR runs an **ACP server** embedded in the Tauri backend. Agents connect to it as clients. This means the application controls what context and capabilities are exposed.

## Connection Lifecycle

1. User configures agent endpoints in settings (URL, API key, model)
2. On project open, LiteSkill VR starts the ACP server on a local port
3. Agent connects and receives the **server manifest** listing available tools and context types
4. User initiates chat → application assembles context, sends to agent via ACP
5. Agent responds with text and/or tool calls
6. Application executes tool calls, returns results, streams response to UI
7. On disconnect, session is persisted

## Server Manifest

The ACP server advertises:

```json
{
  "name": "liteskill-vr",
  "version": "0.1.0",
  "capabilities": {
    "tools": true,
    "context": true,
    "streaming": true
  }
}
```

## Exposed Tools

Tools the agent can invoke to interact with project data:

### Project & Findings

| Tool                            | Description                                                    |
| ------------------------------- | -------------------------------------------------------------- |
| `project.get_summary`           | Get project overview, target list, finding counts              |
| `finding.list`                  | List findings with optional filters (severity, status, target) |
| `finding.get(id)`               | Get full finding details                                       |
| `finding.create(data)`          | Create a new finding (draft status)                            |
| `finding.update(id, data)`      | Update finding fields                                          |
| `finding.annotate(id, content)` | Add annotation to a finding                                    |

### Call Graph

| Tool                                        | Description                        |
| ------------------------------------------- | ---------------------------------- |
| `graph.get_node(id)`                        | Get node details and annotations   |
| `graph.get_neighbors(id, direction, depth)` | Get surrounding subgraph           |
| `graph.find_paths(source, sink, max_depth)` | Compute paths between nodes        |
| `graph.search(query)`                       | Search nodes by name/attribute     |
| `graph.annotate_node(id, content)`          | Annotate a node                    |
| `graph.mark_node(id, role)`                 | Mark node as source/sink/sanitizer |

### Evidence & Export

| Tool                                | Description                             |
| ----------------------------------- | --------------------------------------- |
| `evidence.attach(finding_id, data)` | Attach evidence to a finding            |
| `export.report(project_id, format)` | Generate report (markdown, JSON, SARIF) |

## Context Documents

Structured context sent to agents with each message:

```json
{
  "type": "context",
  "documents": [
    {
      "type": "project_summary",
      "content": { ... }
    },
    {
      "type": "active_focus",
      "content": {
        "focus_type": "graph_node",
        "node": { ... },
        "neighbors": [ ... ],
        "linked_findings": [ ... ]
      }
    },
    {
      "type": "session_history",
      "content": { "messages": [ ... ] }
    }
  ]
}
```

## Agent Configuration

Stored in application settings:

```json
{
  "agents": [
    {
      "id": "claude",
      "name": "Claude Code",
      "provider": "anthropic",
      "endpoint": "https://api.anthropic.com",
      "api_key_ref": "keychain:anthropic_api_key",
      "model": "claude-sonnet-4-6",
      "max_context_tokens": 200000
    },
    {
      "id": "codex",
      "name": "Codex",
      "provider": "openai",
      "endpoint": "https://api.openai.com",
      "api_key_ref": "keychain:openai_api_key",
      "model": "codex",
      "max_context_tokens": 128000
    }
  ]
}
```

API keys are stored in the OS keychain, not in config files.

## Security Considerations

- ACP server binds to `127.0.0.1` only — no remote access
- Tool calls that mutate data (create, update, delete) require user confirmation by default
- Agents cannot access the filesystem directly — only through exposed tools
- All agent actions are logged in the session audit trail
- Rate limiting on tool calls to prevent runaway loops
