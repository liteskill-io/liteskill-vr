import { SeverityBadge, StatusBadge } from "@/components/SeverityBadge";
import { useStore } from "@/lib/store";

import type { IoiWithTags } from "@/lib/types";

const severityOrder = ["critical", "high", "medium", "low", "info"] as const;
const severityBarColor: Record<string, string> = {
  critical: "bg-critical",
  high: "bg-high",
  medium: "bg-medium",
  low: "bg-low",
  info: "bg-info",
};

interface FlatIoi extends IoiWithTags {
  parentName: string;
  parentId: string;
}

export function Dashboard(): React.JSX.Element {
  const items = useStore((s) => s.items);
  const itemDetails = useStore((s) => s.itemDetails);
  const openTab = useStore((s) => s.openTab);
  const showConnectionMap = useStore((s) => s.showConnectionMap);
  const mcpPort = useStore((s) => s.mcpPort);

  const allIois: FlatIoi[] = [];
  for (const item of items) {
    const detail = itemDetails[item.item.id];
    if (!detail) continue;
    for (const ioi of detail.items_of_interest) {
      allIois.push({
        ...ioi,
        parentName: item.item.name,
        parentId: item.item.id,
      });
    }
  }

  const severityCounts: Record<string, number> = {};
  const statusCounts: Record<string, number> = {};
  for (const ioi of allIois) {
    if (ioi.severity) {
      severityCounts[ioi.severity] = (severityCounts[ioi.severity] ?? 0) + 1;
    }
    statusCounts[ioi.status] = (statusCounts[ioi.status] ?? 0) + 1;
  }

  const totalIoi = allIois.length;
  const totalNotes = items.reduce((sum, i) => sum + i.note_count, 0);
  const totalConnections = items.reduce(
    (sum, i) => sum + i.connection_count,
    0,
  );
  const reviewedCount = items.filter(
    (i) => i.item.analysis_status === "reviewed",
  ).length;
  const inProgressCount = items.filter(
    (i) => i.item.analysis_status === "in_progress",
  ).length;
  const untouchedCount = items.filter(
    (i) => i.item.analysis_status === "untouched",
  ).length;

  const criticalAndHigh = allIois.filter(
    (ioi) =>
      (ioi.severity === "critical" || ioi.severity === "high") &&
      ioi.status !== "false_positive",
  );

  const recentIois = allIois
    .toSorted((a, b) => b.created_at.localeCompare(a.created_at))
    .slice(0, 10);

  if (items.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4 text-text-dim">
        <div className="text-4xl font-black tracking-tight text-border select-none">
          LITESKILL
        </div>
        <div className="max-w-sm text-center text-xs leading-relaxed">
          MCP server running on{" "}
          <span className="font-mono text-accent">
            {mcpPort ? `127.0.0.1:${String(mcpPort)}` : "starting…"}
          </span>
        </div>
        <div className="text-[10px] text-text-dim/50">
          Connect an agent and start documenting.
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col overflow-y-auto">
      {/* Stats bar */}
      <div className="grid shrink-0 grid-cols-5 gap-px border-b border-border bg-border">
        <StatCard label="Items" value={items.length} />
        <StatCard label="Findings" value={totalIoi} />
        <StatCard label="Notes" value={totalNotes} />
        <StatCard
          label="Connections"
          value={totalConnections}
          onClick={showConnectionMap}
        />
        <StatCard
          label="Reviewed"
          value={`${String(reviewedCount)}/${String(items.length)}`}
        />
      </div>

      <div className="flex min-h-0 flex-1">
        {/* Left column */}
        <div className="flex w-72 shrink-0 flex-col border-r border-border">
          {/* Severity breakdown */}
          <div className="border-b border-border p-3">
            <div className="mb-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
              Severity Breakdown
            </div>
            {totalIoi > 0 && (
              <div className="mb-3 flex h-2 overflow-hidden rounded-sm">
                {severityOrder.map((sev) => {
                  const count = severityCounts[sev] ?? 0;
                  if (count === 0) return null;
                  const pct = (count / totalIoi) * 100;
                  return (
                    <div
                      key={sev}
                      className={`${severityBarColor[sev] ?? ""} transition-all`}
                      style={{ width: `${String(pct)}%` }}
                    />
                  );
                })}
              </div>
            )}
            <div className="space-y-1">
              {severityOrder.map((sev) => {
                const count = severityCounts[sev] ?? 0;
                return (
                  <div
                    key={sev}
                    className="flex items-center justify-between text-[11px]"
                  >
                    <SeverityBadge severity={sev} />
                    <span className="tabular-nums text-text-dim">{count}</span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Status breakdown */}
          <div className="border-b border-border p-3">
            <div className="mb-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
              Triage Status
            </div>
            <div className="space-y-1">
              {Object.entries(statusCounts)
                .toSorted(([, a], [, b]) => b - a)
                .map(([status, count]) => (
                  <div
                    key={status}
                    className="flex items-center justify-between text-[11px]"
                  >
                    <StatusBadge status={status} />
                    <span className="tabular-nums text-text-dim">{count}</span>
                  </div>
                ))}
            </div>
          </div>

          {/* Analysis progress */}
          <div className="p-3">
            <div className="mb-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
              Analysis Progress
            </div>
            <div className="space-y-1.5 text-[11px]">
              <ProgressRow
                label="Reviewed"
                count={reviewedCount}
                total={items.length}
                color="bg-low"
              />
              <ProgressRow
                label="In Progress"
                count={inProgressCount}
                total={items.length}
                color="bg-accent"
              />
              <ProgressRow
                label="Untouched"
                count={untouchedCount}
                total={items.length}
                color="bg-text-dim/40"
              />
            </div>
          </div>
        </div>

        {/* Right column — findings */}
        <div className="flex min-w-0 flex-1 flex-col">
          {/* Critical & High */}
          {criticalAndHigh.length > 0 && (
            <div className="border-b border-border">
              <div className="px-4 py-2 text-[10px] font-semibold tracking-widest text-critical uppercase">
                Critical & High ({criticalAndHigh.length})
              </div>
              {criticalAndHigh.map((ioi) => (
                <IoiRow key={ioi.id} ioi={ioi} onNavigate={openTab} />
              ))}
            </div>
          )}

          {/* Recent */}
          {recentIois.length > 0 && (
            <div className="flex-1 overflow-y-auto">
              <div className="px-4 py-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
                Recent Findings
              </div>
              {recentIois.map((ioi) => (
                <IoiRow key={ioi.id} ioi={ioi} onNavigate={openTab} />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  onClick,
}: {
  label: string;
  value: number | string;
  onClick?: () => void;
}): React.JSX.Element {
  const inner = (
    <>
      <div className="text-lg font-bold tabular-nums text-text-bright">
        {value}
      </div>
      <div className="text-[9px] font-semibold tracking-widest text-text-dim uppercase">
        {label}
      </div>
    </>
  );
  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        className="bg-surface px-3 py-2.5 text-center transition-colors hover:bg-surface-hover cursor-pointer"
      >
        {inner}
      </button>
    );
  }
  return <div className="bg-surface px-3 py-2.5 text-center">{inner}</div>;
}

function ProgressRow({
  label,
  count,
  total,
  color,
}: {
  label: string;
  count: number;
  total: number;
  color: string;
}): React.JSX.Element {
  const pct = total > 0 ? (count / total) * 100 : 0;
  return (
    <div>
      <div className="flex justify-between text-text-dim">
        <span>{label}</span>
        <span className="tabular-nums">
          {count}/{total}
        </span>
      </div>
      <div className="mt-0.5 h-1 overflow-hidden rounded-sm bg-border">
        <div
          className={`h-full ${color} transition-all`}
          style={{ width: `${String(pct)}%` }}
        />
      </div>
    </div>
  );
}

function IoiRow({
  ioi,
  onNavigate,
}: {
  ioi: FlatIoi;
  onNavigate: (id: string) => void;
}): React.JSX.Element {
  return (
    <button
      type="button"
      onClick={(): void => {
        onNavigate(ioi.parentId);
      }}
      className="flex w-full items-start gap-2 border-t border-border px-4 py-2 text-left transition-colors hover:bg-surface-hover"
    >
      <div className="mt-0.5 flex shrink-0 gap-1">
        <SeverityBadge severity={ioi.severity} />
        <StatusBadge status={ioi.status} />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-[11px] font-medium font-mono text-text-bright">
            {ioi.title}
          </span>
          {ioi.location && (
            <span className="shrink-0 text-[9px] text-text-dim font-mono">
              @ {ioi.location}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2 text-[9px] text-text-dim">
          <span className="font-mono text-accent-dim">{ioi.parentName}</span>
          {ioi.tags.length > 0 && <span>{ioi.tags.join(", ")}</span>}
        </div>
      </div>
    </button>
  );
}
