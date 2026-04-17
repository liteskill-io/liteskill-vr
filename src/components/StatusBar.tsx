import { useStore } from "@/lib/store";

export function StatusBar(): React.JSX.Element {
  const items = useStore((s) => s.items);
  const itemDetails = useStore((s) => s.itemDetails);
  const mcpPort = useStore((s) => s.mcpPort);

  let totalIoi = 0;
  let criticalCount = 0;
  let highCount = 0;
  for (const item of items) {
    const detail = itemDetails[item.item.id];
    if (!detail) continue;
    for (const ioi of detail.items_of_interest) {
      if (ioi.status === "false_positive") continue;
      totalIoi++;
      if (ioi.severity === "critical") criticalCount++;
      if (ioi.severity === "high") highCount++;
    }
  }

  return (
    <div className="flex shrink-0 items-center gap-4 border-t border-border bg-surface px-3 py-1 text-[10px] text-text-dim">
      <span>
        MCP <span className="text-low">●</span>{" "}
        {mcpPort ? `127.0.0.1:${String(mcpPort)}` : "starting…"}
      </span>
      <span>{items.length} items</span>
      <span>{totalIoi} findings</span>
      {criticalCount > 0 && (
        <span className="text-critical">{criticalCount} critical</span>
      )}
      {highCount > 0 && <span className="text-high">{highCount} high</span>}
    </div>
  );
}
