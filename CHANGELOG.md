# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html):
- **MAJOR** for breaking changes or a public release milestone
- **MINOR** for a new module or user-facing capability
- **PATCH** for fixes

## [Unreleased]

### Added
- Planning docs: `docs/PLAN.md`, `docs/VERIFICATION.md`, `docs/DECISIONS.md`, `docs/TEST-MATRIX.md`.
- Pass / warning / fail status badges before each status headline (`.badge`, `badge()` in `src/main.js`).

### Security
- Escape backend strings (reader and profile names, paths, tool errors) before rendering, so a crafted device or profile name can't inject script that calls Tauri commands.
- Strict Content Security Policy instead of none.
- `register_nss` refuses any module path other than the one `detect_middleware` validated.

### Fixed
- `CardPresent` shows a warning badge and "A card is responding." instead of "Card readable." until P2.1 can tell a CAC from a SIM reader (VER-025).
- Hint and path text contrast: `#888` → `#767676` (4.54:1 on white).
- Button and input border contrast: `#ccc` → `#8f8f8f` (3.2:1 on white).

### Planned releases (see docs/PLAN.md)
| Version | Phase | Contents |
|---|---|---|
| 0.1.1 | 0 | The scaffold compiles and runs on Windows and macOS (build and API-drift fixes only) |
| 0.1.2 | 1 | Verified constants (DoD root thumbprints, DISA URL, middleware paths) |
| 0.2.0 | 2 | Layered reader/middleware/trust outcomes (PIV + legacy CAC card check), Windows check-and-guide to DISA InstallRoot, macOS native file picker with `.p7b`/`.zip` parsing and install |
| 0.3.0 | 3 | Firefox diagnostic replaces `modutil` registration (**breaking:** `register_nss` removed) |
| 0.4.0 | 4 | Browser extension + Native Messaging host |
| 0.5.0 | 5 | Unified "Check everything" guided setup |
| 0.9.0 | 6 | Signed and notarized installers, auto-update (first public beta) |
| 1.0.0 | 7 | Public release after the hardware beta |

## [0.1.0] - 2026-10-21

### Added
- Scaffold for modules 1–4 (never compiled):
  - Reader detection (PC/SC)
  - Middleware detection with a `C_GetFunctionList` check
  - DoD root trust check and a scoped-elevation cert install
  - Firefox profile discovery with `modutil` registration
