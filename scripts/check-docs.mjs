#!/usr/bin/env node
// Freshness guard for CLAUDE.md (and its AGENTS.md symlink): fail if the doc
// references a `task` target that no longer exists or links a repo-relative path
// that no longer resolves. Keeps the agent guidance honest without duplicating
// content. Zero dependencies — invoked via `task docs:check`.
//
// To avoid false positives it only inspects backtick code spans (not prose) and
// only treats a code span as a path when it contains a "/" and a file extension
// (bare basenames like `store.ts` are descriptive, not links).
import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const doc = "CLAUDE.md";
const text = readFileSync(resolve(root, doc), "utf8");
const codeSpans = [...text.matchAll(/`([^`]+)`/g)].map((m) => m[1]);
const errors = [];

// 1. Every `task <name>` mentioned in a code span must exist in Taskfile.yml.
const taskfile = readFileSync(resolve(root, "Taskfile.yml"), "utf8");
const definedTasks = new Set(
  [...taskfile.matchAll(/^ {2}([a-z][\w:-]*):/gm)].map((m) => m[1]),
);
const referencedTasks = new Set();
for (const span of codeSpans) {
  for (const m of span.matchAll(/\btask\s+([a-z][\w:-]*)/g))
    referencedTasks.add(m[1]);
}
for (const name of referencedTasks) {
  if (!definedTasks.has(name)) {
    errors.push(
      `references \`task ${name}\` but no such target in Taskfile.yml`,
    );
  }
}

// 2. Every repo-relative path must resolve — path-shaped code spans (contain "/"
//    and an extension) plus markdown links [text](path).
const pathTargets = new Set();
for (const span of codeSpans) {
  if (/^[\w./-]+$/.test(span) && span.includes("/") && /\.\w+$/.test(span)) {
    pathTargets.add(span);
  }
}
for (const m of text.matchAll(/\]\(([^)]+)\)/g)) {
  const t = m[1];
  if (!/^(https?:|mailto:|#)/.test(t)) pathTargets.add(t);
}
for (const target of pathTargets) {
  const path = target.split("#")[0].replace(/^\.\//, "");
  if (!path || path.includes("*")) continue;
  if (!existsSync(resolve(root, path))) {
    errors.push(`links \`${target}\` but ${path} does not exist`);
  }
}

if (errors.length > 0) {
  console.error(`${doc} is stale:`);
  for (const e of errors) console.error(`  - ${e}`);
  console.error("\nFix the reference, or update CLAUDE.md.");
  process.exit(1);
}
console.log(
  `${doc} OK — ${referencedTasks.size} task refs and ${pathTargets.size} path links resolve.`,
);
