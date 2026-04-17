import { invoke } from "@tauri-apps/api/core";

import type { ProjectSnapshot } from "./types";

export async function getSnapshot(): Promise<ProjectSnapshot> {
  return invoke("project_snapshot");
}
