# Overview

## Product Vision

LiteSkill VR is a structured notebook for reverse engineering research. It is not a binary analysis tool — it stores no files, runs no analysis, renders no disassembly. It is a documentation layer where humans and AI agents record what they find using other tools (Ghidra, IDA, command line, etc.).

The app hosts an MCP server so AI agents like Claude Code or Codex can read and write research data directly. The researcher works in their existing tools and uses LiteSkill VR as the persistent structured store that ties everything together.

## Goals

- **Structured research database**: Projects contain items (representing binaries, scripts, configs — any target of interest), each with notes, items of interest, and connections to other items. No actual files are stored — items are documentation entries.
- **MCP server**: The app hosts an MCP server on localhost. AI agents connect to it to query and populate research data. No built-in chat — the researcher uses Claude Code, Codex, or any MCP-capable agent externally.
- **Read-write for agents**: Agents can query all existing research, review past sessions' findings, search for patterns, and build on prior analysis. The read side is as important as the write side.
- **Open-ended connections**: Items can be connected with typed or freeform relationships. A connection map visualizes how items relate across the project.
- **Registered tags**: Tags must be registered in the project before use. A starter set of common vuln classes is provided. Agents must search existing tags before creating new ones.
- **Searchability**: Full-text search across all notes, items of interest, connections, and metadata in a project.
- **One database per project**: Each project is a standalone SQLite file. Easy to back up, share, or archive.
- **Offline-first**: All research data is stored locally.

## Non-Goals

- Not a binary analysis tool — no disassembly, no decompilation, no file storage.
- Not a chat application — no built-in AI chat.
- Not a Ghidra plugin — Ghidra stays separate.
- Not a fuzzer, scanner, or exploit framework.

## Typical Workflow

1. Researcher opens LiteSkill VR and creates a project for a firmware image.
2. Researcher adds items to the project via the UI (or the agent does it via MCP). Items are documentation entries representing files of interest — no actual files are imported.
3. Researcher opens Ghidra with the binaries loaded (pyghidra-mcp running).
4. Researcher starts Claude Code with two MCP servers configured:
   - `pyghidra-mcp` — reads decompiled code, function lists, xrefs from Ghidra
   - `liteskill-mcp` — reads/writes structured findings to LiteSkill VR
5. Claude analyzes binaries via Ghidra, writes items of interest, notes, and connections into LiteSkill VR.
6. Researcher tabs to LiteSkill VR to review findings, navigate the connection map, search for patterns, and add their own notes.
7. Research accumulates over multiple sessions. Each new Claude session calls `project_summary` to orient itself, then picks up where the last session left off.

## Target Users

- Security researchers performing manual vulnerability analysis
- Red team operators documenting engagement findings
- Bug bounty hunters structuring their methodology

## Tech Stack

| Layer        | Technology                                          |
| ------------ | --------------------------------------------------- |
| Shell        | Tauri v2 (Rust backend)                             |
| Frontend     | TypeScript, React, Tailwind CSS                     |
| State        | Zustand (client), SQLite (one database per project) |
| Graphs       | Cytoscape.js (connection map visualization)         |
| AI Interface | MCP server (hosted by Rust backend)                 |
