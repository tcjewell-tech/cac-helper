# CAC Helper (scaffold)

Unofficial, cross-platform assistant for getting a CAC/smart card reader working
on a personal Windows or macOS machine. Not endorsed by DoD — orchestrates
official DISA cert/driver sources rather than replacing them.

## What's implemented in this scaffold

**Module 1: Reader detection** (`src-tauri/src/reader.rs`)
Queries the OS-level PC/SC stack (winscard on Windows, PCSC framework on
macOS) to determine one of four states:
- No reader found
- Reader found, no card inserted
- Card inserted but unreadable (usually a middleware problem)
- Card present and readable

This is deliberately scoped to hardware/driver-layer detection only — it
does not check DoD cert trust, which is module 3.

**Module 2: Middleware detection** (`src-tauri/src/middleware.rs`)
Checks known install locations for common CAC middleware (OpenSC, HID
ActivClient, Thales SafeNet) and, if found, dynamically loads the library
and calls `C_GetFunctionList` — the one function every PKCS#11 module
must export — to confirm it's a real, correctly-built module rather than
just a leftover file from a partial install. Reports one of:
- No middleware found
- Found but invalid (corrupt install, architecture mismatch)
- Valid (and which module, so later steps know which one to register)

This module does not download or bundle any middleware itself — it only
detects what's present and points the user to the official vendor/DISA
source when something's missing, per the "unofficial helper, not a
software distributor" design constraint.

**Module 3: DoD PKI trust store** (`src-tauri/src/cert_store.rs`)
Read-only check: looks for known DoD root CA common names in the OS trust
store (Windows Root store via `certutil`, macOS System keychain via
`security`) and reports not-installed / partially-installed / installed.

Install flow is deliberately human-in-the-loop and split from the check:
this app never downloads or bundles DoD certs itself. It opens the
official DISA cert page in the user's browser, the user downloads and
picks the file(s) themselves, and only then does the app shell out to
`certutil -addstore` (Windows) or `security add-trusted-cert` (macOS) —
after an explicit in-app confirmation dialog naming the exact files.

**Elevation solved without running the app as admin**: both platform
commands require admin/root, but rather than elevating the whole app
(bad UX for modules 1/2/4, which don't need it), each triggers its own
native OS prompt scoped to just this one action:
- Windows: `certutil` is launched via PowerShell's
  `Start-Process -Verb RunAs -PassThru`, which shows a single UAC prompt
  and passes certutil's real exit code back (1223 = user canceled)
- macOS: the `security` command runs through `osascript ... with
  administrator privileges`, which shows the native Touch ID/password
  dialog

**Still needs real-machine testing before shipping**: the macOS path
nests two layers of string escaping (shell quoting, then AppleScript
string-literal quoting) — reasoned through but not yet run against a
path containing spaces or unusual characters. Test with a real DISA
cert bundle path before relying on it.

**Also verify before shipping**: the hardcoded DoD root CA names and the
DISA cert page URL in `cert_store.rs` were written from memory, not
pulled from a live source — confirm both against current DISA
documentation.

**Module 4: Firefox NSS registration** (`src-tauri/src/firefox_nss.rs`)
Firefox keeps its own certificate/module database per profile, separate
from the OS trust store — so a middleware module the OS already sees
(module 2) isn't automatically visible inside Firefox. This module:
- Finds Firefox profiles by reading `profiles.ini` (the same source
  Firefox itself uses, more reliable than guessing folder names)
- Checks each profile's `pkcs11.txt` for the module path (read-only)
- Registers the module via `modutil -add ... -libfile ... -dbdir sql:...`,
  gated behind an in-app confirmation that reminds the user to close
  Firefox first (the NSS database can't be modified while Firefox holds it)

**Known gaps to solve before this ships**, flagged directly in the code:
- `modutil` ships with NSS tools, not with Firefox itself — this app will
  need to bundle it (with its shared-library dependencies) or require a
  separate install, similar to the middleware verification problem in
  module 2
- The `pkcs11.txt` read-only check format is unverified against a real
  profile — confirm it before relying on it for anything beyond a rough
  diagnostic

## Prerequisites to build

- Rust (`rustup`) — https://rustup.rs
- Tauri CLI: `cargo install tauri-cli --version "^2"`
- Windows: no extra PC/SC setup needed (winscard.dll ships with the OS)
- macOS: no extra PC/SC setup needed (PCSC.framework ships with the OS)
- Linux (not yet a target, but for reference): `pcsc-lite` + `libpcsclite-dev`

## Running it

```
cd cac-helper
cargo tauri dev
```

## Roadmap

The full plan is in [docs/PLAN.md](docs/PLAN.md). The decisions behind it are in
[docs/DECISIONS.md](docs/DECISIONS.md), and open assumptions are in
[docs/VERIFICATION.md](docs/VERIFICATION.md).

| Phase | Goal | Version |
|---|---|---|
| 0 | Make the scaffold compile and run on Windows and macOS | 0.1.1 |
| 1 | Verification sprint (DoD root thumbprints, DISA URL, middleware paths, OS-native CAC support) | 0.1.2 |
| 2 | Harden modules 1–3: layered outcomes, native file picker, `.p7b`/`.zip` parsing, roots → Root / intermediates → CA, macOS install without osascript | 0.2.0 |
| 3 | Firefox: replace `modutil` registration with a read-only diagnostic. Current Firefox uses OS client certs and OS trust by default | 0.3.0 |
| 4 | Browser extension (Chrome, Edge, Firefox) + Native Messaging host | 0.4.0 |
| 5 | Unified "Check everything" guided setup | 0.5.0 |
| 6 | Code signing (Azure Artifact Signing / Apple Developer ID + notarization), installers, auto-update via GitHub Releases | 0.9.0 |
| 7 | Beta testing on real hardware, then public release | 1.0.0 |

> Note: the module notes above describe the 0.1.0 scaffold. Module 4's
> `modutil` approach is being replaced in Phase 3 (see ADR-001), and the
> macOS osascript install path in Phase 2 (see ADR-004a).

## Distribution notes

Because this touches root cert stores, code signing is not optional —
unsigned installers will get flagged or blocked outright, and users have
no way to verify the binary wasn't tampered with. Budget for an Apple
Developer ID and a Windows code-signing cert before wide release.
