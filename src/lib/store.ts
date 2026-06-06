import { create } from "zustand";

import type {
  ConnectionType,
  ExplanationDetail,
  ExplanationSummary,
  ItemDetail,
  ItemSummary,
  ProjectSnapshot,
  Tag,
} from "./types";

type RootView = "dashboard" | "connections" | "explanations" | "vocabulary";

export interface FormField {
  name: string;
  label: string;
  type: "text" | "textarea" | "select" | "checkbox" | "tags" | "number";
  options?: { value: string; label: string }[];
  required?: boolean;
  placeholder?: string;
}

// A modal create/edit form. `tool` is the MCP tool invoked via mcp_call; `hidden`
// values (ids, parent refs) are merged into the submitted args.
export interface FormDesc {
  title: string;
  tool: string;
  submitLabel?: string;
  fields: FormField[];
  initial?: Record<string, unknown>;
  hidden?: Record<string, unknown>;
}

interface ConfirmDesc {
  title: string;
  message: string;
  tool: string;
  args: Record<string, unknown>;
}

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
  explanations: ExplanationSummary[];
  explanationDetails: Record<string, ExplanationDetail>;
  mcpPort: number | null;

  openTabs: string[];
  activeTab: string | null;
  // Which root view to render when no item tab is active.
  rootView: RootView;
  // Selected explanation when rootView === "explanations" (null = list view).
  selectedExplanation: string | null;

  // Global modal layer for create/edit forms and delete confirms.
  activeForm: FormDesc | null;
  confirm: ConfirmDesc | null;

  // App-wide zoom, applied via CSS `zoom` on <body>. Persisted per-install.
  zoom: number;

  applySnapshot: (snapshot: ProjectSnapshot) => void;
  openTab: (id: string) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string | null) => void;
  showDashboard: () => void;
  showConnectionMap: () => void;
  showExplanations: () => void;
  openExplanation: (id: string) => void;
  showVocabulary: () => void;
  openForm: (form: FormDesc) => void;
  closeForm: () => void;
  openConfirm: (confirm: ConfirmDesc) => void;
  closeConfirm: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
}

export const useStore = create<AppState>((set) => ({
  items: [],
  itemDetails: {},
  tags: [],
  connectionTypes: [],
  explanations: [],
  explanationDetails: {},
  mcpPort: null,
  openTabs: [],
  activeTab: null,
  rootView: "dashboard",
  selectedExplanation: null,
  activeForm: null,
  confirm: null,
  zoom: loadZoom(),

  applySnapshot: (snapshot): void => {
    const itemDetails: Record<string, ItemDetail> = {};
    for (const detail of snapshot.details) {
      itemDetails[detail.item.id] = detail;
    }
    const explanationDetails: Record<string, ExplanationDetail> = {};
    for (const detail of snapshot.explanation_details) {
      explanationDetails[detail.id] = detail;
    }
    set({
      items: snapshot.items,
      itemDetails,
      tags: snapshot.tags,
      connectionTypes: snapshot.connection_types,
      explanations: snapshot.explanations,
      explanationDetails,
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

  showExplanations: (): void => {
    set({
      activeTab: null,
      rootView: "explanations",
      selectedExplanation: null,
    });
  },

  openExplanation: (id): void => {
    set({ activeTab: null, rootView: "explanations", selectedExplanation: id });
  },

  showVocabulary: (): void => {
    set({ activeTab: null, rootView: "vocabulary" });
  },

  openForm: (form): void => {
    set({ activeForm: form });
  },
  closeForm: (): void => {
    set({ activeForm: null });
  },
  openConfirm: (confirm): void => {
    set({ confirm });
  },
  closeConfirm: (): void => {
    set({ confirm: null });
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
