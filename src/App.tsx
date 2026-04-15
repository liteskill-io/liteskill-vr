import { useEffect, useCallback } from "react";

import { Dashboard } from "@/components/Dashboard";
import { ItemDetail } from "@/components/ItemDetail";
import { Sidebar } from "@/components/Sidebar";
import { TabBar } from "@/components/TabBar";
import { listItems, listTags, listConnectionTypes, getItem } from "@/lib/ipc";
import { useStore } from "@/lib/store";

function App(): React.JSX.Element {
  const activeTab = useStore((s) => s.activeTab);
  const setItems = useStore((s) => s.setItems);
  const setTags = useStore((s) => s.setTags);
  const setConnectionTypes = useStore((s) => s.setConnectionTypes);
  const setItemDetail = useStore((s) => s.setItemDetail);

  const refresh = useCallback((): void => {
    listItems()
      .then((items) => {
        setItems(items);
        for (const item of items) {
          getItem(item.item.id)
            .then((detail) => {
              setItemDetail(item.item.id, detail);
            })
            .catch(console.error);
        }
      })
      .catch(console.error);
    listTags().then(setTags).catch(console.error);
    listConnectionTypes().then(setConnectionTypes).catch(console.error);
  }, [setItems, setTags, setConnectionTypes, setItemDetail]);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    return (): void => {
      clearInterval(interval);
    };
  }, [refresh]);

  return (
    <div className="flex h-full">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TabBar />
        <div className="flex-1 overflow-hidden">
          {activeTab ? <ItemDetail id={activeTab} /> : <Dashboard />}
        </div>
        <StatusBar />
      </div>
    </div>
  );
}

function StatusBar(): React.JSX.Element {
  const items = useStore((s) => s.items);
  const itemDetails = useStore((s) => s.itemDetails);

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
        MCP <span className="text-low">●</span> 127.0.0.1:27182
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

export default App;
