import { useStore } from "@/lib/store";

import type { ItemDetail, ItemSummary } from "@/lib/types";

const statusDot: Record<string, string> = {
  untouched: "bg-text-dim/40",
  in_progress: "bg-accent",
  reviewed: "bg-low",
};

const severityOrder = ["critical", "high", "medium", "low", "info"] as const;
const severityColor: Record<string, string> = {
  critical: "text-critical",
  high: "text-high",
  medium: "text-medium",
  low: "text-low",
  info: "text-info",
};

function groupBySeverity(
  items: ItemSummary[],
  itemDetails: Record<string, ItemDetail>,
): Record<string, { item: ItemSummary; count: number }[]> {
  const groups: Record<string, { item: ItemSummary; count: number }[]> = {};
  for (const sev of severityOrder) {
    groups[sev] = [];
  }

  for (const item of items) {
    const detail = itemDetails[item.item.id];
    if (!detail) continue;
    const counts: Record<string, number> = {};
    for (const ioi of detail.items_of_interest) {
      if (ioi.severity && ioi.status !== "false_positive") {
        counts[ioi.severity] = (counts[ioi.severity] ?? 0) + 1;
      }
    }
    for (const [sev, count] of Object.entries(counts)) {
      groups[sev]?.push({ item, count });
    }
  }
  return groups;
}

export function Sidebar(): React.JSX.Element {
  const items = useStore((s) => s.items);
  const activeTab = useStore((s) => s.activeTab);
  const openTab = useStore((s) => s.openTab);
  const setActiveTab = useStore((s) => s.setActiveTab);
  const itemDetails = useStore((s) => s.itemDetails);

  const groups = groupBySeverity(items, itemDetails);
  const hasFindings = Object.values(groups).some((g) => g.length > 0);

  return (
    <div className="flex h-full w-60 shrink-0 flex-col border-r border-border bg-surface">
      {/* Brand */}
      <div className="flex shrink-0 items-center gap-2 border-b border-border px-3 py-2">
        {/* Served from public/ as a real file, not a Vite data URI —
            data-URI SVGs rasterized oddly in WebKitGTK at small sizes. */}
        <img
          src="/liteskill_vr_app_icon_mono_light.svg"
          alt=""
          aria-hidden="true"
          width={24}
          height={24}
          className="shrink-0 rounded-sm"
        />
        <span className="text-[10px] font-bold tracking-[0.2em] uppercase text-text-bright">
          LiteSkill VR
        </span>
      </div>

      {/* Dashboard link */}
      <button
        type="button"
        onClick={(): void => {
          setActiveTab(null);
        }}
        className={`w-full border-b border-border px-3 py-2.5 text-left text-[10px] font-bold tracking-[0.2em] uppercase transition-colors hover:bg-surface-hover ${
          activeTab === null ? "text-accent" : "text-text-dim"
        }`}
      >
        ◆ Dashboard
      </button>

      {/* Severity groups */}
      {hasFindings && (
        <div className="border-b border-border">
          <div className="px-3 py-1.5 text-[9px] font-semibold tracking-widest text-text-dim uppercase">
            By Severity
          </div>
          {severityOrder.map((sev) => {
            const group = groups[sev] ?? [];
            if (group.length === 0) return null;
            const totalCount = group.reduce((sum, g) => sum + g.count, 0);
            return (
              <div key={sev} className="mb-0.5">
                <div
                  className={`flex items-center justify-between px-3 py-1 text-[10px] font-semibold uppercase ${severityColor[sev] ?? "text-text-dim"}`}
                >
                  <span>{sev}</span>
                  <span className="tabular-nums">{totalCount}</span>
                </div>
                {group.map(({ item: s, count }) => (
                  <button
                    key={`${sev}-${s.item.id}`}
                    type="button"
                    onClick={(): void => {
                      openTab(s.item.id);
                    }}
                    className={`flex w-full items-center justify-between px-4 py-0.5 text-left text-[11px] transition-colors hover:bg-surface-hover ${
                      activeTab === s.item.id
                        ? "text-text-bright bg-surface-hover"
                        : "text-text-dim"
                    }`}
                  >
                    <span className="truncate font-mono">{s.item.name}</span>
                    <span className="shrink-0 tabular-nums text-[10px]">
                      {count}
                    </span>
                  </button>
                ))}
              </div>
            );
          })}
        </div>
      )}

      {/* All items */}
      <div className="flex-1 overflow-y-auto">
        <div className="px-3 py-1.5 text-[9px] font-semibold tracking-widest text-text-dim uppercase">
          All Items ({items.length})
        </div>
        {items.length === 0 && (
          <div className="px-3 py-3 text-text-dim text-[10px]">
            No items yet.
          </div>
        )}
        {items.map((s) => (
          <button
            key={s.item.id}
            type="button"
            onClick={(): void => {
              openTab(s.item.id);
            }}
            className={`flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors hover:bg-surface-hover ${
              activeTab === s.item.id
                ? "bg-surface-hover text-text-bright"
                : "text-text"
            }`}
          >
            <span
              className={`h-1.5 w-1.5 shrink-0 rounded-full ${statusDot[s.item.analysis_status] ?? "bg-text-dim/30"}`}
            />
            <span className="min-w-0 flex-1 truncate text-[11px] font-mono">
              {s.item.name}
            </span>
            {s.ioi_count > 0 && (
              <span className="shrink-0 text-[9px] text-text-dim tabular-nums">
                {s.ioi_count}
              </span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
