import {
  bulkDeleteForm,
  connectionTypeCreateForm,
  tagCreateForm,
} from "@/lib/forms";
import { useStore } from "@/lib/store";

export function Vocabulary(): React.JSX.Element {
  const tags = useStore((s) => s.tags);
  const connectionTypes = useStore((s) => s.connectionTypes);
  const openForm = useStore((s) => s.openForm);
  const openConfirm = useStore((s) => s.openConfirm);

  return (
    <div className="h-full overflow-y-auto p-5">
      <h1 className="mb-1 text-lg font-semibold text-text-bright">
        Vocabularies
      </h1>
      <p className="mb-4 text-[12px] text-text-dim">
        Tags and connection types must be registered before use — full CRUD
        parity with the agent tools.
      </p>

      <section className="mb-6">
        <div className="mb-1 flex items-center justify-between">
          <h2 className="text-[10px] font-semibold tracking-widest text-text-dim uppercase">
            Tags ({tags.length})
          </h2>
          <button
            type="button"
            className="text-[11px] text-accent hover:underline"
            onClick={(): void => {
              openForm(tagCreateForm());
            }}
          >
            + New tag
          </button>
        </div>
        {tags.map((t) => (
          <div
            key={t.id}
            className="flex items-center gap-2 border-b border-border py-1 text-[12px]"
          >
            <span
              className="h-2.5 w-2.5 shrink-0 rounded-full border border-border"
              style={t.color != null ? { background: t.color } : undefined}
            />
            <span className="font-mono text-text">{t.name}</span>
            <span className="flex-1 truncate text-text-dim">
              {t.description}
            </span>
            <button
              type="button"
              className="shrink-0 text-[11px] text-text-dim hover:text-critical"
              onClick={(): void => {
                openConfirm({
                  title: "Delete tag",
                  message: `Delete tag "${t.name}"? It will be removed from all entities.`,
                  tool: "tag_delete",
                  args: { id: t.id },
                });
              }}
            >
              delete
            </button>
          </div>
        ))}
      </section>

      <section className="mb-6">
        <div className="mb-1 flex items-center justify-between">
          <h2 className="text-[10px] font-semibold tracking-widest text-text-dim uppercase">
            Connection types ({connectionTypes.length})
          </h2>
          <button
            type="button"
            className="text-[11px] text-accent hover:underline"
            onClick={(): void => {
              openForm(connectionTypeCreateForm());
            }}
          >
            + New type
          </button>
        </div>
        {connectionTypes.map((c) => (
          <div
            key={c.id}
            className="flex items-center gap-2 border-b border-border py-1 text-[12px]"
          >
            <span className="font-mono text-text">{c.name}</span>
            <span className="flex-1 truncate text-text-dim">
              {c.description}
            </span>
            <button
              type="button"
              className="shrink-0 text-[11px] text-text-dim hover:text-critical"
              onClick={(): void => {
                openConfirm({
                  title: "Delete connection type",
                  message: `Delete "${c.name}"? All connections of this type are removed.`,
                  tool: "connection_type_delete",
                  args: { id: c.id },
                });
              }}
            >
              delete
            </button>
          </div>
        ))}
      </section>

      <section>
        <h2 className="mb-1 text-[10px] font-semibold tracking-widest text-critical uppercase">
          Danger zone
        </h2>
        <button
          type="button"
          className="rounded-sm border border-critical/50 px-2 py-1 text-[11px] text-critical hover:bg-critical/10"
          onClick={(): void => {
            openForm(bulkDeleteForm());
          }}
        >
          Bulk delete…
        </button>
      </section>
    </div>
  );
}
