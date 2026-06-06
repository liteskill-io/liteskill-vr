#!/usr/bin/env node
// Human/agent parity gate.
//
// Core requirement: there must be zero things an AI agent can do that a human
// cannot do in the UI. MUTATION_TOOLS (src-tauri/src/mcp/server.rs) is the
// source of truth for what an agent can write. A mutating tool is considered
// "covered" by the UI when its name appears as a literal in the frontend source
// (src/**) — i.e. some component invokes it via mcp_call. This fails the build
// if any mutating tool lacks a UI affordance.
//
// The *_batch tools are allowlisted: a human creating entries one at a time
// reaches the same end state, so the capability is covered by the singular tool.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// 1. Parse MUTATION_TOOLS from server.rs.
const serverRs = readFileSync(
  resolve(root, "src-tauri/src/mcp/server.rs"),
  "utf8",
);
const block = serverRs.match(/MUTATION_TOOLS[^=]*=\s*&\[([\s\S]*?)\];/);
if (!block) {
  console.error("Could not find MUTATION_TOOLS in server.rs");
  process.exit(2);
}
const tools = [...block[1].matchAll(/"([a-z_]+)"/g)].map((m) => m[1]);

// Allowlisted: batch creates are covered by their singular equivalents.
const allowlist = new Set([
  "item_create_batch",
  "note_create_batch",
  "ioi_create_batch",
  "connection_create_batch",
]);

// 2. Gather all frontend source text.
function walk(dir) {
  let out = "";
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const s = statSync(p);
    if (s.isDirectory()) out += walk(p);
    else if (/\.(ts|tsx)$/.test(name)) out += readFileSync(p, "utf8");
  }
  return out;
}
const uiSource = walk(resolve(root, "src"));

// 3. A tool is covered if the UI references its name as a string literal.
const missing = tools
  .filter((t) => !allowlist.has(t))
  .filter((t) => !uiSource.includes(`"${t}"`));

const checked = tools.filter((t) => !allowlist.has(t)).length;
if (missing.length > 0) {
  console.error(
    `Human/agent parity FAILED — ${missing.length}/${checked} mutating tools have no UI affordance:`,
  );
  for (const t of missing) console.error(`  - ${t}`);
  console.error(
    "\nEvery mutating MCP tool must be reachable from the UI via mcp_call.",
  );
  process.exit(1);
}
console.log(
  `Human/agent parity OK — all ${checked} mutating tools have a UI affordance (${allowlist.size} batch tools allowlisted).`,
);
