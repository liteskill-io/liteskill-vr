import type { KnipConfig } from "knip";

const config: KnipConfig = {
  project: ["src/**/*.{ts,tsx}"],
  ignoreDependencies: [
    // Tauri runtime deps — used via IPC, not direct imports (yet)
    "@tauri-apps/api",
    "@tauri-apps/plugin-opener",
  ],
};

export default config;
