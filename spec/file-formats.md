# File Formats & Interoperability

## Design Principle

All exported data should be self-describing: a human can read the file and understand the research, and an AI agent can parse it without custom instructions.

## Export Formats

### Markdown Report

Human-readable report generated from project data.

```markdown
# [Project Name] — Vulnerability Research Report

## Target: [target name]

- Type: source
- Location: https://github.com/example/repo

### Finding: Buffer Overflow in parse_header (Critical)

- **CWE**: CWE-787 (Out-of-bounds Write)
- **CVSS**: 9.8 (CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H)
- **Status**: confirmed

#### Description

[markdown narrative]

#### Technical Detail

[reproduction steps, PoC code blocks]

#### Evidence

- screenshot: crash.png
- log: asan_output.txt

#### Call Graph Path

source_func → parse_input → parse_header → memcpy (sink)
```

### JSON Export

Machine-readable full project export conforming to the data model schemas.

```json
{
  "format": "liteskill-vr",
  "version": "1.0",
  "exported_at": "2026-04-13T00:00:00Z",
  "project": { ... },
  "targets": [ ... ],
  "findings": [ ... ],
  "call_graphs": [ ... ],
  "sessions": [ ... ]
}
```

### SARIF

Static Analysis Results Interchange Format — for integration with GitHub Security, Azure DevOps, and other tools that consume SARIF.

- Each finding maps to a SARIF `result`
- Call graph paths map to SARIF `codeFlows`
- Evidence maps to SARIF `attachments`

## Import Formats

### Call Graphs

See [call-graph.md](call-graph.md) for supported import sources (Ghidra, IDA, CodeQL, manual, custom JSON).

### SARIF Import

Import findings from other static analysis tools as draft findings for review.

### LiteSkill VR JSON

Re-import previously exported projects for merging or archiving.

## File Association

`.lsvr` extension registered with the OS for project files (JSON format with the extension renamed). Double-clicking opens the project in LiteSkill VR.
