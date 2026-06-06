import type { KnipConfig } from "knip";

const config: KnipConfig = {
  project: ["src/**/*.{ts,tsx}"],
  // `task` (taskfile.dev) is the external task runner some package.json scripts
  // delegate to; it is not an npm dependency.
  ignoreBinaries: ["task"],
  ignoreDependencies: [
    // Tauri plugin loaded at runtime, not imported in TS
    "@tauri-apps/plugin-opener",
    // WebdriverIO — loaded by the runner, not imported directly
    "@wdio/local-runner",
    "webdriverio",
  ],
};

export default config;
