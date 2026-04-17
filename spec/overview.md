# Overview

## Product Vision

LiteSkill VR is a structured notebook for reverse engineering research. It stores no files, runs no analysis, and renders no disassembly. It is a documentation layer where humans and AI agents record what they find using other tools (Ghidra, IDA, command line, etc.).

The app hosts an MCP server on localhost so AI agents like Claude Code or Codex can read and write research data alongside the researcher.

## Goals

- **Structured research database**: Projects contain items (representing binaries, scripts, configs — any subject of analysis), each with notes, items of interest, and connections to other items.
- **Read-write MCP interface**: Agents query existing research, review past sessions, search for patterns, and build on prior analysis. Batch operations and structured filters support serious work at scale.
- **Registered vocabularies**: Tags and connection types must be registered before use, preventing drift across sessions. A starter set of common vulnerability classes is provided.
- **Searchability**: Full-text search and structured filtering across all entities in a project.
- **One database per project**: Each project is a standalone `.lsvr` file (SQLite). Easy to back up, share, or archive.
- **Offline-first**: All data stored locally.

## Non-Goals

- Not a binary analysis tool — no disassembly, no decompilation, no file storage.
- Not a chat application — no built-in AI chat. Use Claude Code or Codex externally.
- Not a Ghidra plugin — Ghidra stays separate. Agents bridge the two via MCP.
- Not a fuzzer, scanner, or exploit framework.

## Typical Workflow

1. Researcher opens LiteSkill VR and creates a project.
2. Researcher (or agent via MCP) adds items representing files of interest.
3. Researcher opens Ghidra with binaries loaded (pyghidra-mcp running).
4. Researcher starts Claude Code configured with both `pyghidra-mcp` and `liteskill-mcp`.
5. Claude calls `project_summary` to orient, then analyzes via Ghidra and documents into LiteSkill VR.
6. Researcher reviews, edits, and annotates in the UI.
7. Next session: Claude calls `changes_since` to catch up, then continues where the last session left off.

On headless machines (CI, servers, remote boxes) the same MCP interface is
available as a standalone `liteskillvr-mcp` binary that runs directly against a
`.lsvr` file with no GUI, WebKitGTK, or Tauri dependencies. See
[architecture.md](architecture.md#headless-mcp-binary) and
[mcp.md](mcp.md#headless-binary).

## Target Users

- Security researchers performing manual vulnerability analysis
- Red team operators documenting engagement findings
- Bug bounty hunters structuring their methodology

## Tech Stack

| Layer        | Technology                                              |
| ------------ | ------------------------------------------------------- |
| Shell        | Tauri v2 (Rust backend)                                 |
| Frontend     | TypeScript, React, Tailwind CSS                         |
| State        | Zustand (client), SQLite (one `.lsvr` file per project) |
| Graphs       | Cytoscape.js (connection map visualization)             |
| AI Interface | MCP server (hosted by Rust backend)                     |
