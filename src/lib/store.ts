import { create } from "zustand";

import type {
  ConnectionType,
  ItemDetail,
  ItemSummary,
  ProjectSnapshot,
  Tag,
} from "./types";

interface AppState {
  items: ItemSummary[];
  itemDetails: Record<string, ItemDetail>;
  tags: Tag[];
  connectionTypes: ConnectionType[];
  mcpPort: number | null;

  openTabs: string[];
  activeTab: string | null;

  applySnapshot: (snapshot: ProjectSnapshot) => void;
  openTab: (id: string) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string | null) => void;
}

export const useStore = create<AppState>((set) => ({
  items: [],
  itemDetails: {},
  tags: [],
  connectionTypes: [],
  mcpPort: null,
  openTabs: [],
  activeTab: null,

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
}));
