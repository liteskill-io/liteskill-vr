import { create } from "zustand";

import type { ItemDetail, ItemSummary, Tag, ConnectionType } from "./types";

interface AppState {
  items: ItemSummary[];
  tags: Tag[];
  connectionTypes: ConnectionType[];
  openTabs: string[];
  activeTab: string | null;
  itemDetails: Record<string, ItemDetail>;

  setItems: (items: ItemSummary[]) => void;
  setTags: (tags: Tag[]) => void;
  setConnectionTypes: (types: ConnectionType[]) => void;
  setItemDetail: (id: string, detail: ItemDetail) => void;
  openTab: (id: string) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string | null) => void;
}

export const useStore = create<AppState>((set) => ({
  items: [],
  tags: [],
  connectionTypes: [],
  openTabs: [],
  activeTab: null,
  itemDetails: {},

  setItems: (items): void => {
    set({ items });
  },
  setTags: (tags): void => {
    set({ tags });
  },
  setConnectionTypes: (types): void => {
    set({ connectionTypes: types });
  },

  setItemDetail: (id, detail): void => {
    set((state) => ({
      itemDetails: { ...state.itemDetails, [id]: detail },
    }));
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
