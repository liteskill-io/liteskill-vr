import { SeverityBadge, StatusBadge } from "@/components/SeverityBadge";
import { useStore } from "@/lib/store";

export function ItemDetail({ id }: { id: string }): React.JSX.Element {
  const detail = useStore((s) => s.itemDetails[id]);
  const itemDetails = useStore((s) => s.itemDetails);
  const items = useStore((s) => s.items);
  const openTab = useStore((s) => s.openTab);

  if (!detail) {
    return (
      <div className="flex h-full items-center justify-center text-text-dim">
        Loading...
      </div>
    );
  }

  const { item, notes, items_of_interest, connections } = detail;

  function resolveEntityName(entityId: string, entityType: string): string {
    if (entityType === "item") {
      const found = items.find((i) => i.item.id === entityId);
      return found?.item.name ?? entityId.slice(0, 8);
    }
    for (const d of Object.values(itemDetails)) {
      const ioi = d.items_of_interest.find((i) => i.id === entityId);
      if (ioi) return ioi.title;
    }
    return entityId.slice(0, 8);
  }

  return (
    <div className="flex h-full flex-col overflow-y-auto">
      {/* Header */}
      <div className="shrink-0 border-b border-border px-4 py-3">
        <div className="flex items-center gap-3">
          <h1 className="text-sm font-semibold text-text-bright">
            {item.name}
          </h1>
          <span className="text-[10px] text-text-dim uppercase">
            {item.item_type}
          </span>
          <span className="text-[10px] text-text-dim">
            {item.analysis_status.replace("_", " ")}
          </span>
        </div>
        {item.path && (
          <div className="mt-1 text-[10px] text-text-dim font-mono">
            {item.path}
          </div>
        )}
        {item.tags.length > 0 && (
          <div className="mt-1.5 flex gap-1">
            {item.tags.map((t) => (
              <span
                key={t}
                className="rounded bg-accent-dim/30 px-1.5 py-0.5 text-[10px] text-accent"
              >
                {t}
              </span>
            ))}
          </div>
        )}
        {item.description && (
          <div className="mt-2 text-xs text-text-dim whitespace-pre-wrap">
            {item.description}
          </div>
        )}
      </div>

      {/* IOIs */}
      {items_of_interest.length > 0 && (
        <div className="border-b border-border">
          <div className="px-4 py-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
            Items of Interest ({items_of_interest.length})
          </div>
          {items_of_interest.map((ioi) => (
            <div
              key={ioi.id}
              className="border-t border-border px-4 py-2 hover:bg-surface-hover transition-colors"
            >
              <div className="flex items-center gap-2">
                <SeverityBadge severity={ioi.severity} />
                <StatusBadge status={ioi.status} />
                <span className="text-xs font-medium text-text-bright font-mono">
                  {ioi.title}
                </span>
                {ioi.location && (
                  <span className="text-[10px] text-text-dim font-mono">
                    @ {ioi.location}
                  </span>
                )}
              </div>
              {ioi.description && (
                <div className="mt-1 text-xs text-text-dim whitespace-pre-wrap">
                  {ioi.description}
                </div>
              )}
              {ioi.tags.length > 0 && (
                <div className="mt-1 flex gap-1">
                  {ioi.tags.map((t) => (
                    <span
                      key={t}
                      className="rounded bg-accent-dim/30 px-1 py-0.5 text-[9px] text-accent"
                    >
                      {t}
                    </span>
                  ))}
                </div>
              )}
              <div className="mt-1 text-[9px] text-text-dim">
                {ioi.author_type === "agent" ? "🤖" : "👤"} {ioi.author}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Notes */}
      {notes.length > 0 && (
        <div className="border-b border-border">
          <div className="px-4 py-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
            Notes ({notes.length})
          </div>
          {notes.map((note) => (
            <div
              key={note.id}
              className="border-t border-border px-4 py-2 hover:bg-surface-hover transition-colors"
            >
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium text-text-bright">
                  {note.title}
                </span>
                <span className="text-[9px] text-text-dim">
                  {note.author_type === "agent" ? "🤖" : "👤"} {note.author}
                </span>
              </div>
              <div className="mt-1 text-xs text-text whitespace-pre-wrap">
                {note.content}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Connections */}
      {connections.length > 0 && (
        <div>
          <div className="px-4 py-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
            Connections ({connections.length})
          </div>
          {connections.map((conn) => {
            const isSource = conn.source_id === id;
            const otherId = isSource ? conn.target_id : conn.source_id;
            const otherType = isSource ? conn.target_type : conn.source_type;
            const otherName = resolveEntityName(otherId, otherType);
            const direction = isSource ? "→" : "←";

            return (
              <button
                key={conn.id}
                type="button"
                onClick={(): void => {
                  if (otherType === "item") openTab(otherId);
                }}
                className="flex w-full items-center gap-2 border-t border-border px-4 py-2 text-left hover:bg-surface-hover transition-colors"
              >
                <span className="text-[10px] text-accent font-mono">
                  {conn.connection_type}
                </span>
                <span className="text-[10px] text-text-dim">{direction}</span>
                <span className="text-xs text-text-bright font-mono">
                  {otherName}
                </span>
                {conn.description && (
                  <span className="text-[10px] text-text-dim truncate">
                    — {conn.description}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      )}

      {/* Empty state */}
      {items_of_interest.length === 0 &&
        notes.length === 0 &&
        connections.length === 0 && (
          <div className="flex flex-1 items-center justify-center text-text-dim text-xs">
            No findings yet. Use MCP to add items of interest, notes, and
            connections.
          </div>
        )}
    </div>
  );
}
