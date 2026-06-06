import { invoke } from "@tauri-apps/api/core";

import type { ProjectSnapshot } from "./types";

export async function getSnapshot(): Promise<ProjectSnapshot> {
  return invoke("project_snapshot");
}

// The single human write path: runs the same MCP tool dispatch agents use,
// stamped author_type="human". The backend emits db-changed on success, which
// triggers a snapshot refetch in App.tsx — so callers don't update state by hand.
export async function mcpCall(
  tool: string,
  args: Record<string, unknown>,
): Promise<unknown> {
  return invoke("mcp_call", { tool, args });
}
