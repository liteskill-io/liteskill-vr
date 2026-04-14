# Security Considerations

## Threat Model

LiteSkill VR handles sensitive vulnerability research data. The threat model considers:

**No at-rest encryption is provided by the software itself. This must be done at the OS level (LUKS, FileVault, BitLocker, etc.).**

| Threat                      | Mitigation                                                                        |
| --------------------------- | --------------------------------------------------------------------------------- |
| Malicious agent output      | Agent-created content is rendered as sanitized markdown. No raw HTML execution.   |
| MCP server exposure         | Bound to 127.0.0.1 only. Not accessible from the network.                         |
| Malicious graph imports     | Import parsers validate and sanitize input. No code execution from imported data. |
| Supply chain (dependencies) | Tauri's Rust backend minimizes JS dependency surface. CSP enforced in webview.    |

## Data Handling

- All project data stored locally in SQLite — no cloud sync
- Export files may contain sensitive vulnerability details — user is warned before export
- All MCP mutations are logged with author, author_type, and timestamp

## Content Security Policy

The Tauri webview enforces a strict CSP:

- No inline scripts
- No eval
- No external resource loading (fonts, images loaded from app bundle)
- connect-src limited to localhost (MCP server)
