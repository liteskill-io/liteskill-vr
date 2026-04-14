# Overview

## Product Vision

LiteSkill VR is a desktop application for vulnerability researchers to methodically document their findings in a structured, reproducible format. Documentation is dual-purpose: human-readable for reports and collaboration, and machine-readable so AI agents (Claude Code, Codex) can ingest, query, and contribute to research via ACP.

## Goals

- **Firmware-scale analysis**: First-class support for firmware images containing multiple binaries and `.so` files, with cross-binary call graphs, shared symbol tracking, and whole-firmware attack surface mapping.
- **Rapid navigation**: Browser-like back/forward history, keyboard-driven jump-to-function/address, bookmarks, and a command palette. Researchers hop between binaries and functions without losing context.
- **Structured documentation**: Consistent schema for documenting vulnerabilities, attack surfaces, call graphs, proof-of-concept steps, and findings.
- **AI-assisted research**: Integrate AI agents through ACP to assist with analysis, suggest attack vectors, annotate code, and generate reports.
- **Call graph analysis**: Visualize and annotate call graphs — within a single binary and across binary boundaries — to trace data flow from sources to sinks.
- **Dual readability**: All persisted data must be meaningful to both humans (via the UI and exported reports) and AI agents (via structured formats and ACP context).
- **Offline-first**: All research data is stored locally. Network access is only required for AI agent communication.
- **Extensible**: Plugin-friendly architecture for adding new analysis modules, importers, and exporters.

## Non-Goals

- Not a fuzzer, scanner, or exploit framework — this is a documentation and analysis tool.
- Not a collaborative real-time editor (single-user desktop app; export/import for sharing).
- Not a replacement for IDEs — integrates with them via file references and ACP, but does not provide code editing.

## Target Users

- Security researchers performing manual vulnerability analysis
- Red team operators documenting engagement findings
- Bug bounty hunters structuring their methodology
- AI agents operating as research assistants via ACP

## Tech Stack

| Layer    | Technology                                              |
| -------- | ------------------------------------------------------- |
| Shell    | Tauri v2 (Rust backend)                                 |
| Frontend | TypeScript, React, Tailwind CSS                         |
| State    | Zustand (client), SQLite via Tauri plugin (persistence) |
| Graphs   | D3.js or Cytoscape.js (call graph visualization)        |
| AI Comms | ACP (Agent Communication Protocol)                      |
| Formats  | Markdown, JSON, SARIF (import/export)                   |
