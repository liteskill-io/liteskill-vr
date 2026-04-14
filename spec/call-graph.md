# Call Graph Analysis

## Purpose

The call graph viewer lets researchers visualize, navigate, and annotate function call relationships within a target. It is the primary surface for tracing data flow from attacker-controlled sources to security-sensitive sinks.

## Import Sources

| Source  | Format                                | Notes                                       |
| ------- | ------------------------------------- | ------------------------------------------- |
| Ghidra  | JSON export (via script)              | Function call trees, xrefs                  |
| IDA Pro | JSON / IDAPython export               | Call graph + metadata                       |
| CodeQL  | SARIF / CSV                           | Query results as graphs                     |
| Manual  | UI or JSON                            | Hand-drawn graphs for API/protocol analysis |
| Custom  | JSON conforming to `CallGraph` schema | Any tool that exports to the schema         |

A Tauri command `graph_import` accepts format + payload and normalizes into the internal `CallGraph` schema.

## Visualization

### Rendering

- Graph rendered using Cytoscape.js (preferred for large graphs) or D3-force
- Layout algorithms: hierarchical (default for call trees), force-directed (for exploration), dagre (for data flow)
- Nodes are colored by attribute:
  - Red: marked as **sink** (security-sensitive function)
  - Green: marked as **source** (attacker-controlled input)
  - Yellow: on an active **path** between source and sink
  - Blue: has **annotations**
  - Gray: default / unclassified

### Interaction

- **Click** node: select, show properties in bottom panel
- **Double-click** node: expand/collapse callees
- **Right-click** node: context menu
  - Mark as source / sink
  - Annotate
  - Trace paths to/from this node
  - Link to finding
  - Ask agent about this function
- **Shift+click**: multi-select for batch operations
- **Scroll**: zoom; **drag canvas**: pan; **drag node**: reposition
- **Search**: filter nodes by name, address, or attribute

### Path Tracing

Core analysis feature: find all paths between a source and a sink.

1. User marks one or more nodes as **source** and one or more as **sink**
2. Invokes "Trace paths" (toolbar button or context menu)
3. Backend computes all simple paths (with configurable depth limit)
4. Paths are highlighted on the graph and listed in a panel
5. Each path can be annotated or linked to a finding

### Taint Tracking Overlay

Optional overlay that visualizes taint propagation:

- Nodes annotated with taint state (`tainted`, `sanitized`, `unknown`)
- Edges show taint transfer (attribute: `propagates_taint: true/false`)
- Visual: tainted paths drawn with a thicker, colored edge style

## AI Integration

Agents can interact with the call graph via ACP tools:

- `graph.get_node(id)` — retrieve node details
- `graph.get_neighbors(id, direction, depth)` — retrieve surrounding subgraph
- `graph.find_paths(source_id, sink_id, max_depth)` — compute paths
- `graph.annotate_node(id, content)` — add annotation
- `graph.mark_node(id, role)` — mark as source/sink/sanitizer
- `graph.search(query)` — find nodes by name or attribute

This allows agents to autonomously explore the call graph, identify interesting paths, and annotate findings.

## Performance

- Graphs up to ~10,000 nodes rendered client-side with virtualization
- Larger graphs: server-side (Rust) subgraph extraction, only visible portion sent to frontend
- Lazy expansion: collapsed subtrees loaded on demand
