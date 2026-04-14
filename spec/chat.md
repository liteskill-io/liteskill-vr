# Chat Window Specification

## Purpose

The chat panel provides a conversational interface to AI agents for research assistance. Agents receive structured project context via ACP and can read/write project data through exposed tools.

## UI Components

### Agent Selector

Dropdown at the top of the chat panel to select the active agent:

- Each configured ACP agent appears as an option
- Shows connection status (green dot = connected)
- "Configure agents..." link opens settings

### Message History

Scrollable message list displaying the conversation:

- **User messages**: right-aligned, distinct background
- **Agent messages**: left-aligned, supports markdown rendering, code blocks, and inline references to project entities
- **System messages**: centered, muted — connection events, context updates, tool calls
- **Tool call indicators**: collapsible blocks showing what tools the agent invoked and their results
- Messages can reference findings, nodes, and targets via `@mention` syntax rendered as clickable links

### Input Area

- Multi-line text input with markdown support
- `@` autocomplete for referencing project entities (findings, targets, nodes)
- `/` commands for quick actions:
  - `/context` — show what context will be sent to the agent
  - `/clear` — clear chat history (does not delete session record)
  - `/export` — export session as markdown
  - `/focus <node-id>` — set graph focus and update agent context
- Shift+Enter for newline, Enter to send
- Attachment button to include evidence or code snippets inline

## Context Management

The chat system automatically assembles context for the agent based on the user's current focus:

| User Focus                 | Context Included                                                         |
| -------------------------- | ------------------------------------------------------------------------ |
| Call graph node selected   | Node properties, neighboring nodes (1-hop), linked findings, annotations |
| Finding open in editor     | Full finding data, linked nodes, evidence summaries                      |
| Target selected in sidebar | Target metadata, finding summaries, graph overview                       |
| No specific focus          | Project summary, recent activity, open findings                          |

Users can manually override context via the `/context` command or by pinning specific items.

## Session Persistence

- Every chat interaction is recorded as a `ResearchSession`
- Sessions include all messages and a log of any mutations the agent made
- Sessions can be resumed or reviewed later
- Session history is available to agents as prior conversation context

## Streaming

Agent responses are streamed token-by-token via ACP streaming. The UI renders incrementally with a typing indicator. Tool calls are displayed as they execute, not batched at the end.
