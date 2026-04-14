# LiteSkill VR

Desktop application for methodical vulnerability research documentation. Built with Tauri v2, React, and TypeScript.

See [spec/](spec/) for detailed design documents.

## Development

```bash
pnpm install
pnpm tauri dev
```

## Scripts

| Script               | Purpose                                                      |
| -------------------- | ------------------------------------------------------------ |
| `pnpm dev`           | Start Vite dev server                                        |
| `pnpm tauri dev`     | Start Tauri dev mode                                         |
| `pnpm check`         | Run all checks (typecheck, lint, oxlint, format, knip, test) |
| `pnpm check:rust`    | Run Rust checks (clippy, fmt)                                |
| `pnpm check:all`     | Run both TS and Rust checks                                  |
| `pnpm test`          | Run unit tests                                               |
| `pnpm test:e2e`      | Run E2E tests (requires release build)                       |
| `pnpm test:coverage` | Run tests with coverage                                      |

## Building

```bash
# Release binary
APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=1 pnpm tauri build

# Binary only (no .deb/.rpm/.AppImage bundles)
pnpm tauri build --no-bundle
```

## Prerequisites

- Node.js >= 22
- Rust >= 1.77
- [Tauri v2 Linux dependencies](https://tauri.app/start/prerequisites/#linux)
- `WebKitWebDriver` (for E2E tests): `dnf install webkit2gtk4.1-webdriver`
