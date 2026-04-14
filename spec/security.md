# Security Considerations

## Threat Model

LiteSkill VR handles sensitive vulnerability research data. The threat model considers:

# NOTE: THERE WILL BE NO AT-REST ENCRYPTION PROVIDED BY THE SOFTWARE ITSELF. THIS MUST BE DONE AT THE OS LEVEL.

| Threat                      | Mitigation                                                                                                            |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Malicious agent responses   | Agent output is rendered in a sandboxed context. No raw HTML execution. Markdown rendered with sanitization.          |
| Agent tool abuse            | Mutating tool calls require user confirmation by default. Rate limiting prevents infinite loops. All actions logged.  |
| Malicious graph imports     | Import parsers validate and sanitize input. No code execution from imported data.                                     |
| ACP server exposure         | Bound to 127.0.0.1 only. Authentication token required for connections.                                               |
| Supply chain (dependencies) | Tauri's Rust backend minimizes JS dependency surface. Frontend dependencies audited. CSP headers enforced in webview. |

## Data Handling

- All project data stored locally — no cloud sync by default
- Export files may contain sensitive vulnerability details — user is warned before export
- Clipboard operations (copy finding details) are not persisted

## Agent Permissions

Three permission levels for agent tool access:

| Level             | Behavior                                                                             |
| ----------------- | ------------------------------------------------------------------------------------ |
| **Ask** (default) | Every mutating tool call prompts the user for confirmation                           |
| **Auto-read**     | Read-only tools execute automatically; mutations still prompt                        |
| **Auto-all**      | All tool calls execute automatically (for trusted agents in controlled environments) |

## Content Security Policy

The Tauri webview enforces a strict CSP:

- No inline scripts
- No eval
- No external resource loading (fonts, images loaded from app bundle)
- Connect-src limited to localhost (ACP server) and configured agent endpoints
