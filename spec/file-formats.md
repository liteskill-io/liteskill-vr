# File Formats

> **Implementation status.** Only the `.lsvr` project file is implemented. Every
> export and import format below is **Planned — not yet built** (there is no
> export/import code in the repo today), and OS file-association is not wired up
> either. This document is the design target for that work.

## Project Files

Each project is a single `.lsvr` file (a SQLite database — WAL mode, foreign
keys on). Projects can be backed up, shared, or archived by copying the file.

> **Implementation status.** The desktop app currently opens (or creates)
> `project.lsvr` in its working directory. **Planned:** an open/new file dialog
> and registering the `.lsvr` extension with the OS so double-clicking opens the
> project in LiteSkill VR.

## Export Formats — Planned

None of the export formats below are implemented yet.

### Markdown Report

Human-readable report generated from project data.

```markdown
# [Project Name] — Research Report

## Item: httpd (ELF, arm32)

**Status**: reviewed
**Path**: /usr/bin/httpd
**Tags**: interesting

### Items of Interest

#### parse_header() [Critical] [memory-corruption]

**Location**: 0x08041234

No bounds check on Content-Length header. Attacker-controlled length
passed directly to memcpy.

#### auth_check() [High] [auth-bypass]

**Location**: 0x08042000

Password comparison uses strcmp — timing side-channel.

### Notes

**Session summary** (claude-code, 2024-01-15)

Analyzed httpd binary. Found 2 critical issues in request parsing...

### Connections

- httpd → libfoo.so [links]: dynamically links libfoo.so
- httpd → /etc/httpd.conf [reads_config]: reads config at startup
```

### JSON Export

Machine-readable full project export conforming to the data model.

```json
{
  "format": "liteskill-vr",
  "version": "1.0",
  "exported_at": "2026-04-13T00:00:00Z",
  "project": { ... },
  "tags": [ ... ],
  "connection_types": [ ... ],
  "items": [ ... ],
  "notes": [ ... ],
  "items_of_interest": [ ... ],
  "connections": [ ... ]
}
```

### SARIF

Static Analysis Results Interchange Format — for integration with GitHub Security, Azure DevOps, and other tools.

- Each item of interest maps to a SARIF `result`
- Severity maps to SARIF `level`
- Tags map to SARIF `taxa`

## Import Formats — Planned

Not implemented yet.

### SARIF Import

Import results from static analysis tools as draft items of interest for review.

### LiteSkill VR JSON

Re-import previously exported projects for merging or archiving.
