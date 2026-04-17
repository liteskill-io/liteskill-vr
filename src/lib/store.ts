import { create } from "zustand";

import type {
  ConnectionType,
  ItemDetail,
  ItemSummary,
  ProjectSnapshot,
  Tag,
} from "./types";

type RootView = "dashboard" | "connections";

const ZOOM_MIN = 0.5;
const ZOOM_MAX = 2.0;
const ZOOM_STEP = 0.1;
const ZOOM_STORAGE_KEY = "liteskill.zoom";

function loadZoom(): number {
  if (typeof localStorage === "undefined") return 1;
  const raw = localStorage.getItem(ZOOM_STORAGE_KEY);
  const parsed = raw ? Number(raw) : NaN;
  if (!Number.isFinite(parsed)) return 1;
  return Math.min(Math.max(parsed, ZOOM_MIN), ZOOM_MAX);
}

function persistZoom(z: number): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(ZOOM_STORAGE_KEY, String(z));
}

interface AppState {
  items: ItemSummary[];
  itemDetails: Record<string, ItemDetail>;
  tags: Tag[];
  connectionTypes: ConnectionType[];
  mcpPort: number | null;

  openTabs: string[];
  activeTab: string | null;
  // Which root view to render when no item tab is active.
  rootView: RootView;

  // App-wide zoom, applied via CSS `zoom` on <body>. Persisted per-install.
  zoom: number;

  applySnapshot: (snapshot: ProjectSnapshot) => void;
  openTab: (id: string) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string | null) => void;
  showDashboard: () => void;
  showConnectionMap: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
}

export const useStore = create<AppState>((set) => ({
  items: [],
  itemDetails: {},
  tags: [],
  connectionTypes: [],
  mcpPort: null,
  openTabs: [],
  activeTab: null,
  rootView: "dashboard",
  zoom: loadZoom(),

  applySnapshot: (snapshot): void => {
    const itemDetails: Record<string, ItemDetail> = {};
    for (const detail of snapshot.details) {
      itemDetails[detail.item.id] = detail;
    }
    set({
      items: snapshot.items,
      itemDetails,
      tags: snapshot.tags,
      connectionTypes: snapshot.connection_types,
      mcpPort: snapshot.mcp_port,
    });
  },

  openTab: (id): void => {
    set((state) => ({
      openTabs: state.openTabs.includes(id)
        ? state.openTabs
        : [...state.openTabs, id],
      activeTab: id,
    }));
  },

  closeTab: (id): void => {
    set((state) => {
      const tabs = state.openTabs.filter((t) => t !== id);
      return {
        openTabs: tabs,
        activeTab:
          state.activeTab === id
            ? (tabs[tabs.length - 1] ?? null)
            : state.activeTab,
      };
    });
  },

  setActiveTab: (id): void => {
    set({ activeTab: id });
  },

  showDashboard: (): void => {
    set({ activeTab: null, rootView: "dashboard" });
  },

  showConnectionMap: (): void => {
    set({ activeTab: null, rootView: "connections" });
  },

  zoomIn: (): void => {
    const z = Math.min(
      Math.round((useStore.getState().zoom + ZOOM_STEP) * 10) / 10,
      ZOOM_MAX,
    );
    persistZoom(z);
    set({ zoom: z });
  },
  zoomOut: (): void => {
    const z = Math.max(
      Math.round((useStore.getState().zoom - ZOOM_STEP) * 10) / 10,
      ZOOM_MIN,
    );
    persistZoom(z);
    set({ zoom: z });
  },
  resetZoom: (): void => {
    persistZoom(1);
    set({ zoom: 1 });
  },
}));
