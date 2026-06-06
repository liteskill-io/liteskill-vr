# Security Considerations

## Threat Model

LiteSkill VR handles sensitive vulnerability research data. The threat model considers:

**No at-rest encryption is provided by the software itself. This must be done at the OS level (LUKS, FileVault, BitLocker, etc.).**

| Threat                      | Mitigation                                                                                                                                                                                            |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Malicious agent output      | Agent-created content is currently rendered as **escaped plain text**, so embedded HTML never executes. (When markdown rendering is added it must sanitize and forbid raw HTML — see [ui.md](ui.md).) |
| MCP server exposure         | Bound to 127.0.0.1 only. Not accessible from the network.                                                                                                                                             |
| Malicious imports (planned) | Import is not implemented yet; when added, parsers must validate and sanitize input with no code execution from imported data. See [file-formats.md](file-formats.md).                                |
| Supply chain (dependencies) | Tauri's Rust backend minimizes JS dependency surface. CSP enforced in webview.                                                                                                                        |

## Data Handling

- All project data stored locally in SQLite — no cloud sync
- Every mutation is stamped with `author`, `author_type`, and a timestamp
- **Planned:** export (markdown/JSON/SARIF) will surface a warning that exported files may contain sensitive vulnerability details

## Content Security Policy

The Tauri webview enforces a strict CSP (`src-tauri/tauri.conf.json`):

```
default-src 'self';
img-src 'self' asset: https://asset.localhost;
style-src 'self';
connect-src 'self' http://127.0.0.1
```

- `default-src 'self'` — no inline scripts, no `eval`, no remote script/resource loading.
- `img-src` additionally allows Tauri's local `asset:`/`asset.localhost` protocol (bundled assets), not arbitrary remote images.
- `connect-src` is limited to the app itself and the local MCP server on `127.0.0.1`.

The app's Tauri capabilities are minimal — only `core:default` and
`opener:default` (`src-tauri/capabilities/`).
