# Architecture

## High-Level Diagram

```
┌─────────────────────────────────────────────────────┐
│                   Tauri Window                      │
│                                                     │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Chat    │  │  Call Graph  │  │  Findings    │   │
│  │  Panel   │  │  Viewer      │  │  Editor      │   │
│  └────┬─────┘  └──────┬───────┘  └──────┬───────┘   │
│       │               │                 │           │
│  ┌────┴───────────────┴─────────────────┴────────┐  │
│  │              Frontend State (Zustand)         │  │
│  └────────────────────┬──────────────────────────┘  │
│                       │ IPC (invoke / events)       │
├───────────────────────┼─────────────────────────────┤
│                       │                             │
│  ┌────────────────────┴──────────────────────────┐  │
│  │              Tauri Rust Backend               │  │
│  │                                               │  │
│  │  ┌───────────┐  ┌──────────┐  ┌───────────┐   │  │
│  │  │ Project   │  │ ACP      │  │ Analysis  │   │  │
│  │  │ Store     │  │ Gateway  │  │ Engine    │   │  │
│  │  └─────┬─────┘  └────┬─────┘  └─────┬─────┘   │  │
│  │        │              │              │        │  │
│  │  ┌─────┴─────┐  ┌────┴──────┐  ┌────┴──────┐  │  │
│  │  │ SQLite    │  │ ACP       │  │ Graph     │  │  │
│  │  │           │  │ Transport │  │ Compute   │  │  │
│  │  └───────────┘  └───────────┘  └───────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Component Responsibilities

### Frontend (TypeScript / React)

- **Chat Panel**: Renders conversation with AI agents; sends/receives messages via ACP Gateway.
- **Call Graph Viewer**: Interactive graph visualization of function call relationships, data flow paths, taint tracking annotations.
- **Findings Editor**: Structured form + markdown editor for documenting vulnerabilities, severity, reproduction steps, and evidence.
- **Project Sidebar**: Tree view of targets, findings, and research sessions.
- **Zustand Store**: Client-side state management; syncs with backend via Tauri IPC.

### Backend (Rust / Tauri)

- **Project Store**: CRUD operations on projects, targets, findings, annotations. Persists to SQLite.
- **ACP Gateway**: Manages connections to AI agents via ACP. Handles message routing, context assembly, and tool registration.
- **Analysis Engine**: Computes call graphs from imported data, performs reachability analysis, tracks taint propagation metadata.
- **Graph Compute**: Graph algorithms (shortest path, dominators, strongly connected components) used by the analysis engine.

## Data Flow

1. User creates a **project** containing one or more **targets** (binaries, source repos, APIs).
2. User imports or manually builds **call graphs** for targets.
3. User documents **findings** — each linked to graph nodes, source locations, and evidence.
4. AI agents connect via ACP, receive project context, and can query/annotate findings and graphs.
5. User exports research as Markdown reports, JSON, or SARIF.

## IPC Contract

All frontend-backend communication uses Tauri's `invoke` for request/response and `listen`/`emit` for events.

```
invoke("project_create", { name, description }) → ProjectId
invoke("finding_create", { projectId, data }) → FindingId
invoke("graph_import", { projectId, format, payload }) → GraphId
invoke("acp_send", { agentId, message }) → AcpResponse
listen("acp_stream", callback)  // streamed agent responses
listen("graph_update", callback) // live graph mutations
```
