import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { ConnectionMap } from "@/components/ConnectionMap";
import { Dashboard } from "@/components/Dashboard";
import { ItemDetail } from "@/components/ItemDetail";
import { Sidebar } from "@/components/Sidebar";
import { StatusBar } from "@/components/StatusBar";
import { TabBar } from "@/components/TabBar";
import { getSnapshot } from "@/lib/ipc";
import { useStore } from "@/lib/store";

function App(): React.JSX.Element {
  const activeTab = useStore((s) => s.activeTab);
  const rootView = useStore((s) => s.rootView);
  const zoom = useStore((s) => s.zoom);
  const zoomIn = useStore((s) => s.zoomIn);
  const zoomOut = useStore((s) => s.zoomOut);
  const resetZoom = useStore((s) => s.resetZoom);
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

  // Apply app-wide zoom via CSS `zoom` on <body>. Cheaper than transform:scale
  // because layout reflows naturally and scrollbars adjust.
  useEffect(() => {
    document.body.style.zoom = String(zoom);
  }, [zoom]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (!(e.ctrlKey || e.metaKey)) return;
      // "+" and "=" share a physical key on US layouts — accept either.
      if (e.key === "+" || e.key === "=") {
        e.preventDefault();
        zoomIn();
      } else if (e.key === "-") {
        e.preventDefault();
        zoomOut();
      } else if (e.key === "0") {
        e.preventDefault();
        resetZoom();
      }
    };
    window.addEventListener("keydown", onKey);
    return (): void => {
      window.removeEventListener("keydown", onKey);
    };
  }, [zoomIn, zoomOut, resetZoom]);

  const mainView = activeTab ? (
    <ItemDetail id={activeTab} />
  ) : rootView === "connections" ? (
    <ConnectionMap />
  ) : (
    <Dashboard />
  );

  return (
    <div className="flex h-full">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TabBar />
        <div className="flex-1 overflow-hidden">{mainView}</div>
        <StatusBar />
      </div>
    </div>
  );
}

export default App;
