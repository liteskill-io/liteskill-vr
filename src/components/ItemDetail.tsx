import { SeverityBadge, StatusBadge } from "@/components/SeverityBadge";
import {
  connectionCreateForm,
  ioiCreateForm,
  ioiEditForm,
  itemEditForm,
  noteCreateForm,
  noteEditForm,
} from "@/lib/forms";
import { useStore } from "@/lib/store";

function RowActions({
  onEdit,
  onDelete,
}: {
  onEdit?: () => void;
  onDelete: () => void;
}): React.JSX.Element {
  return (
    <span className="ml-auto flex shrink-0 gap-2">
      {onEdit && (
        <button
          type="button"
          className="text-[10px] text-text-dim hover:text-accent"
          onClick={onEdit}
        >
          edit
        </button>
      )}
      <button
        type="button"
        className="text-[10px] text-text-dim hover:text-critical"
        onClick={onDelete}
      >
        delete
      </button>
    </span>
  );
}

function AddButton({
  label,
  onClick,
}: {
  label: string;
  onClick: () => void;
}): React.JSX.Element {
  return (
    <button
      type="button"
      className="text-[10px] text-accent hover:underline"
      onClick={onClick}
    >
      {label}
    </button>
  );
}

export function ItemDetail({ id }: { id: string }): React.JSX.Element {
  const detail = useStore((s) => s.itemDetails[id]);
  const itemDetails = useStore((s) => s.itemDetails);
  const items = useStore((s) => s.items);
  const connectionTypes = useStore((s) => s.connectionTypes);
  const openTab = useStore((s) => s.openTab);
  const showConnectionMap = useStore((s) => s.showConnectionMap);
  const showDashboard = useStore((s) => s.showDashboard);
  const openForm = useStore((s) => s.openForm);
  const openConfirm = useStore((s) => s.openConfirm);

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
          <RowActions
            onEdit={(): void => {
              openForm(itemEditForm(item));
            }}
            onDelete={(): void => {
              openConfirm({
                title: "Delete item",
                message: `Delete "${item.name}"? Its notes, findings, and connections are removed too.`,
                tool: "item_delete",
                args: { id: item.id },
              });
              showDashboard();
            }}
          />
        </div>
        {item.path && (
          <div className="mt-1 font-mono text-[10px] text-text-dim">
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
          <div className="mt-2 text-xs whitespace-pre-wrap text-text-dim">
            {item.description}
          </div>
        )}
      </div>

      {/* IOIs */}
      <div className="border-b border-border">
        <div className="flex items-center gap-2 px-4 py-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
          <span>Items of Interest ({items_of_interest.length})</span>
          <AddButton
            label="+ Finding"
            onClick={(): void => {
              openForm(ioiCreateForm(id));
            }}
          />
        </div>
        {items_of_interest.map((ioi) => (
          <div key={ioi.id} className="border-t border-border px-4 py-2">
            <div className="flex items-center gap-2">
              <SeverityBadge severity={ioi.severity} />
              <StatusBadge status={ioi.status} />
              <span className="font-mono text-xs font-medium text-text-bright">
                {ioi.title}
              </span>
              {ioi.location && (
                <span className="font-mono text-[10px] text-text-dim">
                  @ {ioi.location}
                </span>
              )}
              <RowActions
                onEdit={(): void => {
                  openForm(ioiEditForm(ioi));
                }}
                onDelete={(): void => {
                  openConfirm({
                    title: "Delete finding",
                    message: `Delete "${ioi.title}"?`,
                    tool: "ioi_delete",
                    args: { id: ioi.id },
                  });
                }}
              />
            </div>
            {ioi.description && (
              <div className="mt-1 text-xs whitespace-pre-wrap text-text-dim">
                {ioi.description}
              </div>
            )}
            <div className="mt-1 text-[9px] text-text-dim">
              {ioi.author_type === "agent" ? "🤖" : "👤"} {ioi.author}
            </div>
          </div>
        ))}
      </div>

      {/* Notes */}
      <div className="border-b border-border">
        <div className="flex items-center gap-2 px-4 py-2 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
          <span>Notes ({notes.length})</span>
          <AddButton
            label="+ Note"
            onClick={(): void => {
              openForm(noteCreateForm(id));
            }}
          />
        </div>
        {notes.map((note) => (
          <div key={note.id} className="border-t border-border px-4 py-2">
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-text-bright">
                {note.title}
              </span>
              <span className="text-[9px] text-text-dim">
                {note.author_type === "agent" ? "🤖" : "👤"} {note.author}
              </span>
              <RowActions
                onEdit={(): void => {
                  openForm(noteEditForm(note));
                }}
                onDelete={(): void => {
                  openConfirm({
                    title: "Delete note",
                    message: `Delete note "${note.title}"?`,
                    tool: "note_delete",
                    args: { id: note.id },
                  });
                }}
              />
            </div>
            <div className="mt-1 text-xs whitespace-pre-wrap text-text">
              {note.content}
            </div>
          </div>
        ))}
      </div>

      {/* Connections */}
      <div>
        <div className="flex items-center gap-2 px-4 py-2 text-[10px] font-semibold tracking-widest uppercase">
          <span className="text-text-dim">
            Connections ({connections.length})
          </span>
          <AddButton
            label="+ Connect"
            onClick={(): void => {
              openForm(connectionCreateForm(id, items, connectionTypes));
            }}
          />
          <button
            type="button"
            onClick={showConnectionMap}
            className="ml-auto text-accent transition-colors hover:text-text-bright"
          >
            ⬡ View in map
          </button>
        </div>
        {connections.map((conn) => {
          const isSource = conn.source_id === id;
          const otherId = isSource ? conn.target_id : conn.source_id;
          const otherType = isSource ? conn.target_type : conn.source_type;
          const otherName = resolveEntityName(otherId, otherType);
          const direction = isSource ? "→" : "←";

          return (
            <div
              key={conn.id}
              className="flex items-center gap-2 border-t border-border px-4 py-2"
            >
              <button
                type="button"
                onClick={(): void => {
                  if (otherType === "item") openTab(otherId);
                }}
                className="flex flex-1 items-center gap-2 text-left"
              >
                <span className="font-mono text-[10px] text-accent">
                  {conn.connection_type}
                </span>
                <span className="text-[10px] text-text-dim">{direction}</span>
                <span className="font-mono text-xs text-text-bright">
                  {otherName}
                </span>
                {conn.description && (
                  <span className="truncate text-[10px] text-text-dim">
                    — {conn.description}
                  </span>
                )}
              </button>
              <RowActions
                onDelete={(): void => {
                  openConfirm({
                    title: "Delete connection",
                    message: "Delete this connection?",
                    tool: "connection_delete",
                    args: { id: conn.id },
                  });
                }}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
