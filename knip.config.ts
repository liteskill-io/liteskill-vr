import type { KnipConfig } from "knip";

const config: KnipConfig = {
  project: ["src/**/*.{ts,tsx}"],
  ignoreDependencies: [
    // Tauri plugin loaded at runtime, not imported in TS
    "@tauri-apps/plugin-opener",
    // WebdriverIO — loaded by the runner, not imported directly
    "@wdio/local-runner",
    "webdriverio",
  ],
};

export default config;
