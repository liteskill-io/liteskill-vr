import type { Options } from "@wdio/types";
import { spawn, type ChildProcess } from "node:child_process";
import path from "node:path";

let tauriDriver: ChildProcess | undefined;

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
        application: path.resolve("./src-tauri/target/release/liteskill-vr"),
      },
    } as WebdriverIO.Capabilities,
  ],

  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 30_000,
  },

  // Start tauri-driver and wait for it to be ready
  onPrepare(): Promise<void> {
    return new Promise((resolve) => {
      tauriDriver = spawn("tauri-driver", [], {
        stdio: ["ignore", "pipe", "pipe"],
      });
      tauriDriver.stderr?.once("data", () => resolve());
      setTimeout(resolve, 2000);
    });
  },

  onComplete(): void {
    tauriDriver?.kill();
  },
};

export { config };
