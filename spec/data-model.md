# Data Model

All entities are stored in SQLite and exposed over Tauri IPC as JSON. Schemas are designed so that raw JSON exports are meaningful to both humans reading them and AI agents ingesting them as context.

## Core Entities

### Project

Top-level container for a research engagement.

```typescript
interface Project {
  id: string; // uuid
  name: string;
  description: string; // markdown
  created_at: string; // ISO 8601
  updated_at: string;
  targets: Target[];
  tags: string[];
  metadata: Record<string, string>; // extensible k/v
}
```

### FirmwareImage

A firmware image containing multiple binaries and shared objects.

```typescript
interface FirmwareImage {
  id: string;
  project_id: string;
  name: string; // e.g. "router_fw_v2.1"
  description: string;
  extraction_path: string; // local path to extracted filesystem
  binaries: Binary[];
  shared_symbols: SharedSymbol[]; // cross-binary symbol map
  created_at: string;
}
```

### Binary

A single ELF, `.so`, kernel module, or other executable within a firmware image. Can also exist standalone (non-firmware projects).

```typescript
interface Binary {
  id: string;
  firmware_id?: string; // null if standalone
  project_id: string;
  name: string; // e.g. "libfoo.so", "httpd"
  type:
    | "executable"
    | "shared_object"
    | "kernel_module"
    | "bootloader"
    | "other";
  path: string; // path within firmware or on disk
  architecture: string; // e.g. "arm32", "mips-le", "x86_64"
  description: string;
  analysis_status: "untouched" | "in_progress" | "reviewed";
  imports: string[]; // imported symbol names
  exports: string[]; // exported symbol names
  call_graphs: CallGraph[];
  findings: Finding[];
}
```

### SharedSymbol

Tracks a symbol shared across binaries (import/export relationships).

```typescript
interface SharedSymbol {
  name: string;
  exported_by: string[]; // binary IDs
  imported_by: string[]; // binary IDs
  type: "function" | "variable";
}
```

### Target (legacy/generic)

For non-firmware targets (source repos, APIs, etc.).

```typescript
interface Target {
  id: string;
  project_id: string;
  name: string;
  type: "source" | "api" | "other";
  location: string;
  description: string;
  call_graphs: CallGraph[];
  findings: Finding[];
}
```

### Finding

A documented vulnerability or observation.

```typescript
interface Finding {
  id: string;
  target_id: string;
  title: string;
  severity: "critical" | "high" | "medium" | "low" | "info";
  status: "draft" | "confirmed" | "reported" | "fixed" | "wontfix";
  description: string; // markdown — human narrative
  technical_detail: string; // markdown — reproduction steps, PoC
  evidence: Evidence[];
  affected_nodes: string[]; // call graph node IDs
  cwe_ids: string[]; // e.g. ["CWE-787"]
  cvss_vector?: string; // CVSS 3.1 vector string
  created_at: string;
  updated_at: string;
  annotations: Annotation[];
  tags: string[];
}
```

### Evidence

Attachments supporting a finding.

```typescript
interface Evidence {
  id: string;
  finding_id: string;
  type: "screenshot" | "log" | "pcap" | "code_snippet" | "file" | "note";
  label: string;
  content: string; // inline text or base64 for binary
  source_ref?: string; // file path or URL of origin
  created_at: string;
}
```

### NavigationEntry

An entry in the user's history stack for back/forward navigation.

```typescript
interface NavigationEntry {
  id: string;
  session_id: string;
  timestamp: string;
  view_type:
    | "firmware_overview"
    | "call_graph"
    | "function_detail"
    | "finding"
    | "xref_map"
    | "diff";
  binary_id?: string;
  node_id?: string;
  finding_id?: string;
  view_state: Record<string, unknown>; // zoom, scroll, selections
}
```

### Bookmark

A saved location for quick access.

```typescript
interface Bookmark {
  id: string;
  project_id: string;
  label: string;
  group?: string; // e.g. "interesting functions", "attack surface"
  binary_id?: string;
  node_id?: string;
  finding_id?: string;
  view_type: string;
  created_at: string;
}
```

### CallGraph

A directed graph representing function/method call relationships. Can span a single binary or cross binary boundaries.

```typescript
interface CallGraph {
  id: string;
  binary_id?: string; // null for cross-binary graphs
  project_id: string;
  name: string;
  scope: "single_binary" | "cross_binary";
  source_format: "ghidra" | "ida" | "codeql" | "manual" | "custom";
  nodes: CallGraphNode[];
  edges: CallGraphEdge[];
  created_at: string;
}

interface CallGraphNode {
  id: string;
  binary_id?: string; // which binary this function belongs to
  label: string; // function name or symbol
  address?: string; // virtual address for binaries
  file_path?: string; // source file for source targets
  line_number?: number;
  is_export: boolean; // is this an exported symbol?
  is_import: boolean; // is this an imported symbol?
  attributes: Record<string, string>; // e.g. { "taint": "source" }
  annotations: Annotation[];
}

interface CallGraphEdge {
  id: string;
  source_node_id: string;
  target_node_id: string;
  type: "call" | "indirect_call" | "callback" | "virtual" | "data_flow";
  label?: string;
  attributes: Record<string, string>;
}
```

### Annotation

A note attached to any entity, created by a human or AI agent.

```typescript
interface Annotation {
  id: string;
  parent_id: string; // finding, node, or edge ID
  parent_type: "finding" | "node" | "edge" | "target";
  author: string; // user name or agent ID
  author_type: "human" | "agent";
  content: string; // markdown
  created_at: string;
}
```

### ResearchSession

A recorded research session (chat history + actions taken).

```typescript
interface ResearchSession {
  id: string;
  project_id: string;
  title: string;
  started_at: string;
  ended_at?: string;
  messages: ChatMessage[];
  actions: SessionAction[]; // log of mutations made during session
}
```

## AI Context Assembly

When an AI agent requests context via ACP, the backend assembles a **context document** from the data model:

```typescript
interface AgentContext {
  project: Project;
  active_target?: Target;
  relevant_findings: Finding[];
  graph_subgraph?: {
    // subgraph around the area of focus
    nodes: CallGraphNode[];
    edges: CallGraphEdge[];
  };
  recent_annotations: Annotation[];
  session_history: ChatMessage[]; // last N messages
}
```

This is serialized as JSON and sent as the system/context block in ACP messages, giving agents full situational awareness.
