import type { Options } from "@wdio/types";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

let tauriDriver: ChildProcess | undefined;

const APP_BINARY = path.resolve("./src-tauri/target/release/liteskill-vr");
const SCREENSHOT_DIR = path.resolve("./e2e/screenshots");
// Runtime dir keeps the test's ./project.lsvr isolated from the repo root,
// so re-running e2e never stomps a developer's real working project file.
const RUNTIME_DIR = path.resolve("./e2e/.runtime");

const config: Options.Testrunner = {
  specs: ["./e2e/**/*.e2e.ts"],
  maxInstances: 1,

  // Connect to tauri-driver — don't auto-manage a browser driver
  hostname: "127.0.0.1",
  port: 4444,

  capabilities: [
    {
      browserName: "wry",
      // Prevent WDIO from injecting webSocketUrl (tauri-driver doesn't support BiDi)
      "wdio:enforceWebDriverClassic": true,
      "tauri:options": {
        application: APP_BINARY,
      },
    } as WebdriverIO.Capabilities,
  ],

  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 60_000,
  },

  onPrepare(): Promise<void> {
    fs.rmSync(RUNTIME_DIR, { recursive: true, force: true });
    fs.mkdirSync(RUNTIME_DIR, { recursive: true });
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });

    return new Promise((resolve) => {
      tauriDriver = spawn("tauri-driver", [], {
        stdio: ["ignore", "pipe", "pipe"],
        // Spawn tauri-driver (and thus the app it launches) in the isolated
        // runtime dir, so `current_dir().join("project.lsvr")` resolves there.
        cwd: RUNTIME_DIR,
      });
      tauriDriver.stderr?.once("data", () => {
        resolve();
      });
      setTimeout(resolve, 2000);
    });
  },

  onComplete(): void {
    tauriDriver?.kill();
  },
};

export { config };
