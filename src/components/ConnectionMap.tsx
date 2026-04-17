import cytoscape from "cytoscape";
import { useEffect, useMemo, useRef, useState } from "react";

import { useStore } from "@/lib/store";

import type { Connection } from "@/lib/types";

const STATUS_BORDER: Record<string, string> = {
  untouched: "#6b7280",
  in_progress: "#4dc9f6",
  reviewed: "#6a9955",
};

const SEVERITY_COLOR: Record<string, string> = {
  critical: "#f44747",
  high: "#e5a84b",
  medium: "#c9a63c",
  low: "#6a9955",
  info: "#569cd6",
};

const LAYOUTS = [
  { id: "cose", label: "Force" },
  { id: "circle", label: "Circle" },
  { id: "concentric", label: "Concentric" },
  { id: "grid", label: "Grid" },
  { id: "breadthfirst", label: "Breadth-first" },
] as const;

type LayoutId = (typeof LAYOUTS)[number]["id"];

interface GraphData {
  nodes: {
    id: string;
    name: string;
    status: string;
    topSeverity: string | null;
    iois: number;
    degree: number;
  }[];
  edges: Connection[];
}

function buildLayoutOptions(
  id: LayoutId,
): cytoscape.LayoutOptions & { name: LayoutId } {
  const common = { padding: 40, animate: false, fit: true } as const;
  switch (id) {
    case "cose":
      return {
        ...common,
        name: "cose",
        idealEdgeLength: () => 160,
        nodeRepulsion: () => 8000,
      };
    case "circle":
      return { ...common, name: "circle", spacingFactor: 1.4 };
    case "concentric":
      return {
        ...common,
        name: "concentric",
        concentric: (n: cytoscape.NodeSingular): number => n.degree(false),
        levelWidth: () => 1,
        spacingFactor: 1.1,
      };
    case "grid":
      return { ...common, name: "grid", spacingFactor: 1.2 };
    case "breadthfirst":
      return {
        ...common,
        name: "breadthfirst",
        spacingFactor: 1.2,
        directed: true,
      };
  }
}

export function ConnectionMap(): React.JSX.Element {
  const items = useStore((s) => s.items);
  const itemDetails = useStore((s) => s.itemDetails);
  const openTab = useStore((s) => s.openTab);
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<cytoscape.Core | null>(null);
  const [layoutId, setLayoutId] = useState<LayoutId>("cose");
  const [hideIsolated, setHideIsolated] = useState(false);

  const graph: GraphData = useMemo(() => {
    const severityRank = ["critical", "high", "medium", "low", "info"];
    const edges = new Map<string, Connection>();
    const degrees = new Map<string, number>();

    // First pass: collect all item-to-item edges.
    for (const it of items) {
      const detail = itemDetails[it.item.id];
      if (!detail) continue;
      for (const conn of detail.connections) {
        if (conn.source_type !== "item" || conn.target_type !== "item")
          continue;
        edges.set(conn.id, conn);
      }
    }
    // Count degree per node.
    for (const e of edges.values()) {
      degrees.set(e.source_id, (degrees.get(e.source_id) ?? 0) + 1);
      degrees.set(e.target_id, (degrees.get(e.target_id) ?? 0) + 1);
    }
    // Second pass: build nodes with top severity.
    const nodes = items.map((it) => {
      const detail = itemDetails[it.item.id];
      let topSeverity: string | null = null;
      if (detail) {
        for (const ioi of detail.items_of_interest) {
          if (ioi.status === "false_positive") continue;
          if (!ioi.severity) continue;
          if (
            topSeverity === null ||
            severityRank.indexOf(ioi.severity) <
              severityRank.indexOf(topSeverity)
          ) {
            topSeverity = ioi.severity;
          }
        }
      }
      return {
        id: it.item.id,
        name: it.item.name,
        status: it.item.analysis_status,
        topSeverity,
        iois: it.ioi_count,
        degree: degrees.get(it.item.id) ?? 0,
      };
    });
    return { nodes, edges: [...edges.values()] };
  }, [items, itemDetails]);

  const visibleNodes = useMemo(
    () =>
      hideIsolated ? graph.nodes.filter((n) => n.degree > 0) : graph.nodes,
    [graph.nodes, hideIsolated],
  );

  useEffect(() => {
    if (!containerRef.current) return;

    const cy = cytoscape({
      container: containerRef.current,
      elements: [
        ...visibleNodes.map((n) => ({
          data: {
            id: n.id,
            label: n.iois > 0 ? `${n.name}  •${String(n.iois)}` : n.name,
            status: n.status,
            fill: n.topSeverity
              ? (SEVERITY_COLOR[n.topSeverity] ?? "#11151c")
              : "#11151c",
            border: STATUS_BORDER[n.status] ?? "#6b7280",
          },
        })),
        ...graph.edges.map((e) => ({
          data: {
            id: e.id,
            source: e.source_id,
            target: e.target_id,
            label: e.connection_type,
          },
        })),
      ],
      style: [
        {
          selector: "node",
          style: {
            "background-color": "data(fill)",
            "background-opacity": 0.25,
            "border-color": "data(border)",
            "border-width": 2,
            label: "data(label)",
            color: "#e8eaed",
            "text-valign": "bottom",
            "text-margin-y": 8,
            "font-family": "JetBrains Mono, monospace",
            "font-size": 11,
            "font-weight": 500,
            width: 48,
            height: 48,
          },
        },
        {
          selector: "node:active, node:selected",
          style: {
            "overlay-opacity": 0,
            "border-width": 3,
          },
        },
        {
          selector: "edge",
          style: {
            width: 1.5,
            "line-color": "#2a3343",
            "curve-style": "bezier",
            "target-arrow-color": "#2a3343",
            "target-arrow-shape": "triangle",
            label: "data(label)",
            "font-family": "JetBrains Mono, monospace",
            "font-size": 9,
            color: "#6b7280",
            "text-background-color": "#0a0e14",
            "text-background-opacity": 1,
            "text-background-padding": "3",
            "text-rotation": "autorotate",
          },
        },
      ],
      layout: buildLayoutOptions(layoutId),
      wheelSensitivity: 0.2,
      minZoom: 0.2,
      maxZoom: 3,
    });

    cy.on("tap", "node", (evt) => {
      const node = evt.target as cytoscape.NodeSingular;
      openTab(node.id());
    });

    cyRef.current = cy;
    return (): void => {
      cy.destroy();
      cyRef.current = null;
    };
  }, [visibleNodes, graph.edges, layoutId, openTab]);

  // Graph-local zoom / fit / re-layout controls — these act on the cytoscape
  // instance directly. App-wide Ctrl+= shortcuts are handled in App.tsx.
  const onZoomIn = (): void => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.zoom({ level: cy.zoom() * 1.25, renderedPosition: viewportCenter(cy) });
  };
  const onZoomOut = (): void => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.zoom({ level: cy.zoom() / 1.25, renderedPosition: viewportCenter(cy) });
  };
  const onFit = (): void => {
    cyRef.current?.fit(undefined, 40);
  };
  const onRelayout = (): void => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.layout(buildLayoutOptions(layoutId)).run();
  };

  if (items.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 text-text-dim">
        <div className="text-[10px] font-semibold tracking-[0.2em] uppercase">
          Connection Map
        </div>
        <div className="text-xs">No items yet — nothing to graph.</div>
      </div>
    );
  }

  const hiddenCount = graph.nodes.length - visibleNodes.length;

  return (
    <div className="flex h-full flex-col">
      <div className="flex shrink-0 items-center justify-between gap-4 border-b border-border px-4 py-2 text-[10px] text-text-dim">
        <span className="font-semibold tracking-[0.2em] uppercase">
          Connection Map — {visibleNodes.length} items · {graph.edges.length}{" "}
          connections
          {hiddenCount > 0 && (
            <span className="ml-2 text-text-dim/60">
              ({hiddenCount} hidden)
            </span>
          )}
        </span>
        <div className="flex items-center gap-1">
          <ToolButton onClick={onZoomIn} title="Zoom in">
            +
          </ToolButton>
          <ToolButton onClick={onZoomOut} title="Zoom out">
            −
          </ToolButton>
          <ToolButton onClick={onFit} title="Fit to view">
            ⬚ Fit
          </ToolButton>
          <ToolButton onClick={onRelayout} title="Re-run layout">
            ↻ Layout
          </ToolButton>
          <select
            value={layoutId}
            onChange={(e): void => {
              setLayoutId(e.target.value as LayoutId);
            }}
            className="rounded border border-border bg-surface px-2 py-0.5 text-[10px] text-text hover:bg-surface-hover"
          >
            {LAYOUTS.map((l) => (
              <option key={l.id} value={l.id}>
                {l.label}
              </option>
            ))}
          </select>
          <label className="ml-2 flex items-center gap-1.5 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={hideIsolated}
              onChange={(e): void => {
                setHideIsolated(e.target.checked);
              }}
              className="h-3 w-3 accent-accent"
            />
            <span>Hide isolated</span>
          </label>
        </div>
      </div>
      <div ref={containerRef} className="min-h-0 flex-1 bg-bg" />
    </div>
  );
}

function viewportCenter(cy: cytoscape.Core): cytoscape.Position {
  const w = cy.width();
  const h = cy.height();
  return { x: w / 2, y: h / 2 };
}

function ToolButton({
  onClick,
  title,
  children,
}: {
  onClick: () => void;
  title: string;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className="rounded border border-border bg-surface px-2 py-0.5 text-[10px] font-semibold text-text-dim hover:bg-surface-hover hover:text-text-bright transition-colors"
    >
      {children}
    </button>
  );
}
