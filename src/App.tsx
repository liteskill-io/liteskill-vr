import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { Dashboard } from "@/components/Dashboard";
import { ItemDetail } from "@/components/ItemDetail";
import { Sidebar } from "@/components/Sidebar";
import { StatusBar } from "@/components/StatusBar";
import { TabBar } from "@/components/TabBar";
import { getSnapshot } from "@/lib/ipc";
import { useStore } from "@/lib/store";

function App(): React.JSX.Element {
  const activeTab = useStore((s) => s.activeTab);
  const applySnapshot = useStore((s) => s.applySnapshot);

  useEffect(() => {
    const refresh = (): void => {
      getSnapshot().then(applySnapshot).catch(console.error);
    };
    refresh();

    const unlisten = listen("db-changed", refresh);
    return (): void => {
      unlisten
        .then((fn) => {
          fn();
        })
        .catch(console.error);
    };
  }, [applySnapshot]);

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

export default App;
