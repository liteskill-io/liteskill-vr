import { useStore } from "@/lib/store";

export function TabBar(): React.JSX.Element {
  const openTabs = useStore((s) => s.openTabs);
  const activeTab = useStore((s) => s.activeTab);
  const setActiveTab = useStore((s) => s.setActiveTab);
  const closeTab = useStore((s) => s.closeTab);
  const items = useStore((s) => s.items);

  if (openTabs.length === 0) return <div />;

  return (
    <div className="flex shrink-0 border-b border-border bg-surface">
      {openTabs.map((id) => {
        const item = items.find((i) => i.item.id === id);
        const name = item?.item.name ?? id.slice(0, 8);
        const isActive = activeTab === id;
        return (
          <button
            key={id}
            type="button"
            onClick={(): void => {
              setActiveTab(id);
            }}
            className={`group flex items-center gap-1.5 border-r border-border px-3 py-1.5 text-xs transition-colors ${
              isActive
                ? "bg-bg text-text-bright"
                : "text-text-dim hover:bg-surface-hover hover:text-text"
            }`}
          >
            <span className="truncate max-w-32">{name}</span>
            <span
              role="button"
              tabIndex={0}
              onClick={(e): void => {
                e.stopPropagation();
                closeTab(id);
              }}
              onKeyDown={(e): void => {
                if (e.key === "Enter") {
                  e.stopPropagation();
                  closeTab(id);
                }
              }}
              className="ml-1 opacity-0 transition-opacity group-hover:opacity-100 hover:text-critical"
            >
              ×
            </span>
          </button>
        );
      })}
    </div>
  );
}
