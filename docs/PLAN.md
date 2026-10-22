# CAC Helper: Implementation Plan v1.0

| Field | Value |
|---|---|
| Date | 2026-10-21 |
| Baseline | `0.1.0`, the scaffold imported unchanged (`chore: import scaffold (modules 1-4)`) |
| Inputs | `CAC-Helper_Claude-Code-Planning-Prompt_v1.0.md` (Drive), [DECISIONS.md](DECISIONS.md), [VERIFICATION.md](VERIFICATION.md), [TEST-MATRIX.md](TEST-MATRIX.md) |

Every task below is sized for one Claude Code session and can be tested on its own.

Each task ID is `P<phase>.<n>`.

**Two owner decisions (2026-10-21) shape this plan:**
1. Module 4 drops `modutil` and becomes a read-only Firefox diagnostic (ADR-001).
2. macOS cert install drops `osascript` and runs `security` as the user (ADR-004a).

## Phase overview

| Phase | Goal | Ships as | Depends on |
|---|---|---|---|
| 0 | The scaffold compiles and runs on Windows and macOS | 0.1.1 | — |
| 1 | Verification sprint: close the register items that block design | 0.1.2 | 0 |
| 2 | Harden modules 1–3: layered outcomes, file picker, `.p7b`/`.zip`, scoped trust | 0.2.0 | 1 |
| 3 | Firefox diagnostic (module 4 redesign) | 0.3.0 | 2 |
| 4 | Browser extension + Native Messaging host | 0.4.0 | 3 |
| 5 | Unified "Check everything" guided flow | 0.5.0 | 4 |
| 6 | Signing, notarization, installers, auto-update | 0.9.0 (first public beta build) | 5 |
| 7 | Beta on real hardware, then 1.0 | 1.0.0 | 6 |

---

## Phase 0: Compile and run (→ 0.1.1)

**Goal.** `cargo check`, `cargo tauri dev` and `cargo tauri build --debug` all succeed on Windows x64 and macOS arm64, and the four cards render with live data. The only changes are build fixes and API drift.

### Compile log (Windows x64, 2026-10-21)
See the [`cargo check` log](#cargo-check-log) at the end of this file. Every error there is a P0 task.

### Tasks
| ID | Task | Files | Acceptance | Test |
|---|---|---|---|---|
| P0.1 | Fix the crate layout. `Cargo.toml` declares `[lib] cac_helper_lib` but there's no `src/lib.rs`. Move the commands and `run()` into `lib.rs`, and have `main.rs` call `cac_helper_lib::run()` (the Tauri v2 template) | `Cargo.toml`, `src/main.rs`, new `src/lib.rs` | `cargo check` gets past target resolution | `cargo check` |
| P0.2 | Add app icons. `bundle.icon: []` breaks the Windows resource step in tauri-build. Generate them with `cargo tauri icon <1024px.png>` | `tauri.conf.json`, `icons/` | No icon error | `cargo check` / `cargo tauri build --debug` |
| P0.3 | Commit `Cargo.lock`, because this is an app and needs reproducible builds. Pin the toolchain with `rust-toolchain.toml`. The Windows check showed no API drift (see the log), so this task covers only fixes that come out of P0.7 on macOS | `Cargo.lock`, `rust-toolchain.toml` | `cargo check` is clean on both OSes | `cargo check` |
| P0.4 | Set `app.withGlobalTauri: true`. Without it `window.__TAURI__` is undefined and every `invoke` fails | `tauri.conf.json` | The four cards show results, not "Error checking …" | `cargo tauri dev`, then a manual look |
| P0.5 | Add a capabilities file `capabilities/default.json` with `core:default` (needed for Tauri v2 core APIs). Custom commands stay allowed | `src-tauri/capabilities/` | No permission errors in the devtools console | `cargo tauri dev` |
| P0.6 | Change the identifier from `helper.cac.app` to one that doesn't end in `.app`, e.g. `io.github.<owner>.cachelper`. Decide this **before** the first signed release, because it changes app-data paths and the updater identity | `tauri.conf.json` | No bundler warning | `cargo tauri build --debug` |
| P0.7 | Make the macOS build run. Do a `cargo check` and a `cargo tauri dev` on the owner's Mac. The `cfg(macos)` code has never been compiled | — | Clean on aarch64-apple-darwin | 🔧 owner's Mac |
| P0.8 | Add CI: GitHub Actions `cargo check` + `cargo clippy` on `windows-latest` and `macos-latest` | `.github/workflows/ci.yml` | A green run on push | GitHub Actions |

---

## Phase 1: Verification sprint (→ 0.1.2)

**Goal.** Close the register items that change design or code. This phase changes no product code. It only updates `VERIFICATION.md` statuses, plus hard-coded constants once they're verified.

| ID | Task | Closes | Test / evidence |
|---|---|---|---|
| P1.1 | Get the current DISA PKCS#7 bundle in a browser. Record the version, its README list of roots, and the SHA-1/SHA-256 thumbprints of DoD Root CA 3/5/6. Update `KNOWN_DOD_ROOTS` to thumbprints and drop Root CA 4 | VER-001, VER-002 | Bundle README + `certutil -dump` output pasted into the register |
| P1.2 | Install OpenSC (current release) in a Windows VM and a macOS VM. Record the real PKCS#11 paths, and those of the other vendors where their docs publish them | VER-003 | Paths + doc links |
| P1.3 | Reproduce UAC cancel behaviour. Record the exit code and stderr of the `Start-Process -Verb RunAs` path | VER-007 | Transcript |
| P1.4 | 🔧 Mac: run `security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain test.cer` **as the user** with a throwaway self-signed CA. Confirm the auth dialog appears and trust is set. Then remove it with `security remove-trusted-cert -d` | VER-021, VER-005 | Screenshot + `security dump-trust-settings -d` |
| P1.5 | 🔧 Clean Windows 11 + CAC: `certutil -scinfo`, list the certs exposed by the inbox minidriver, and try a CAC login in Edge and Firefox | VER-011 | Transcript + result |
| P1.6 | 🔧 Mac + CAC: `sc_auth identities`, `security list-smartcards`, and a login in Safari, Chrome and Firefox | VER-010 | Transcript + result |
| P1.7 | In a Windows VM, run DISA InstallRoot 5.6 and diff the stores (`ROOT`, `CA`, `Disallowed`) before and after | VER-012 | Diff |
| P1.8 | Confirm the Firefox 120 enterprise-roots default from the release notes or pref history. Confirm the Edge NMH paths, the MV3 SW + `sendNativeMessage` behaviour, and Tauri's universal macOS target | VER-014/015/016/018 | Links |

**Exit criteria.** None of VER-001/002/003/007/010/011/012/021 is still `open`. If VER-021 is refuted, Phase 2 uses the Keychain Access hand-off fallback (ADR-004a).

---

## Phase 2: Harden modules 1–3 (→ 0.2.0)

**Goal.** Each check reports the layer that is broken. Cert install takes a picked `.p7b`/`.zip`, previews it, and installs each certificate into the right store with one elevation per batch.

| ID | Task | Files | Acceptance | Test |
|---|---|---|---|---|
| P2.1 | Reader: add `PcscServiceUnavailable` (from `Error::NoService` / `ServiceStopped`). Use `list_readers_len` + owned buffer instead of a fixed 2048 bytes. Ignore the `\\?PnP?\Notification` pseudo-reader. **Fix the false positive found in Phase 0:** this dev PC's built-in cellular SIM reader ("Microsoft UICC ISO Reader") makes the app report "Card readable" with no CAC inserted (VER-025). Exclude UICC readers by name, and only report `CardPresent` after connecting and getting a successful SELECT of the PIV AID (`A0 00 00 03 08`). Otherwise report the new outcome `CardNotCac` | `reader.rs`, `main.js` | Stopping `SCardSvr` on Windows shows the service message. A PC with only a SIM reader reports `NoReaderFound` | Unit tests on outcome mapping (a fake state list) and the reader-name filter, plus manual service stop and a run on this dev PC |
| P2.2 | Middleware: the new outcome model from ADR-002 (`OsNative`, `ThirdPartyValid`, `ThirdPartyInvalid{reason}`, `NothingUsable`). Read the PE/Mach-O header to report `ArchMismatch` before loading. Add OS-native card cert enumeration | `middleware.rs`, `main.js` | A clean Windows 11 + CAC shows `OsNative` | Unit tests for header parsing (fixture DLL/dylib headers). 🔧 for the card path |
| P2.3 | Trust check by thumbprint, covering LocalMachine (and, separately, CurrentUser) `ROOT`/`CA` on Windows and the System keychain plus admin trust settings on macOS. Report expired roots | `cert_store.rs` | `Installed` after InstallRoot in a VM | Unit tests with mocked store listings, plus a VM |
| P2.4 | Bundle parsing with the `cms` + `x509-cert` crates (and `zip`). Return a preview list (subject, issuer, kind, SHA-256, notAfter). Classify root/intermediate/cross-cert | new `src-tauri/src/pki_bundle.rs`, `Cargo.toml` | Parses the real DISA bundle. Counts match `certutil -dump` | Unit tests with a checked-in **synthetic** test bundle. Don't commit DoD certs |
| P2.5 | Native file picker with `tauri-plugin-dialog` (`.p7b`, `.cer`, `.crt`, `.zip`). Remove the typed-path input | `Cargo.toml`, `lib.rs`, `capabilities/default.json`, `index.html`, `main.js` | The user picks a file. The preview shows before the confirmation | Manual |
| P2.6 | Windows install: write the selected certs to temp `.cer` files, then make **one** elevated PowerShell call that runs `certutil -addstore -f ROOT` / `CA` per cert. Map a UAC cancel to the result of P1.3. Delete the temp files | `cert_store.rs` | Roots in ROOT, intermediates in CA. Cancel shows "canceled" | VM: verify the stores before and after |
| P2.7 | macOS install per ADR-004a: argv-based, unprivileged `security`. Remove `shell_single_quote`/`applescript_escape`. Use the fallback if P1.4 failed | `cert_store.rs` | Trust shows in `security dump-trust-settings -d` | 🔧 Mac |
| P2.8 | Make the mutating and blocking commands `async` (`install_certs`, later the Firefox policy write) so UAC/auth waits don't freeze the UI | `lib.rs` | The UI stays responsive during the prompt | Manual |
| P2.9 | Frontend safety: replace `innerHTML` with DOM building/`textContent` (reader names, paths and stderr are untrusted). Set a strict CSP (`default-src 'self'`). Fix the disclaimer, which wrongly says the app fetches certs | `main.js`, `index.html`, `tauri.conf.json` | No `innerHTML` with dynamic data. CSP is active | grep + manual |
| P2.10 | Unit test scaffolding: `cargo test` in CI, and pure functions extracted (outcome mapping, profiles.ini parsing, bundle classification) | all modules, CI | `cargo test` runs in CI | CI |

---

## Phase 3: Firefox diagnostic, module 4 redesign (→ 0.3.0)

**Goal.** Tell the user whether Firefox will use the OS's CAC certs and DoD trust. By default, never modify a profile.

| ID | Task | Files | Acceptance | Test |
|---|---|---|---|---|
| P3.1 | Remove `register_module`/`modutil`, the `register_nss` command and the button | `firefox_nss.rs` → rename `firefox.rs`, `lib.rs`, `main.js`, `index.html` | No `modutil` references left | grep |
| P3.2 | Detect the Firefox install and version (Windows: `HKLM\SOFTWARE\Mozilla\Mozilla Firefox` CurrentVersion / `firefox.exe` file version; macOS: `Info.plist` `CFBundleShortVersionString`), including ESR | `firefox.rs` | Shows the version + "ESR" | Unit tests on parsers + VM |
| P3.3 | Per profile (keep the `profiles.ini` parser, and add `[Install*]` default handling): read `prefs.js`/`user.js` for overrides of `security.osclientcerts.autoload` and `security.enterprise_roots.enabled`. Read `pkcs11.txt` to **report** third-party modules | `firefox.rs` | Outcomes: `Ready`, `PrefDisabled{pref,profile}`, `TooOldForOsTrust{version}`, `NotInstalled` | Fixture profile dirs in tests |
| P3.4 | Detect an enterprise policy that disables these (`policies.json` in the install dir, `HKLM\Software\Policies\Mozilla\Firefox`, the macOS plist) | `firefox.rs` | Reports "managed by policy" | Fixtures |
| P3.5 | Opt-in fix for `TooOldForOsTrust` / ESR: write the `Certificates.ImportEnterpriseRoots` policy (an elevated, confirmed single write, undoable). Show the exact file/key before writing | `firefox.rs`, `main.js` | Undo removes it cleanly | VM |

---

## Phase 4: Browser extension + Native Messaging (→ 0.4.0)

**Goal.** An in-browser "Am I ready for CAC?" panel that talks to the installed app (ADR-005).

| ID | Task | Files | Acceptance | Test |
|---|---|---|---|---|
| P4.1 | NMH binary `cac-helper-nmh`: stdio framing, schema v1, origin allow-list, read-only commands only | `src-tauri/src/bin/nmh.rs`, shared lib | `ping`/`check_all` round-trip with a test harness | Unit tests on framing (length prefix, 1 MB cap) |
| P4.2 | Host manifests + per-user registration (HKCU / `~/Library/...`) for Chrome, Edge and Firefox. Done by the installer, plus a "Repair browser connection" button behind a confirmation | installer hooks, `lib.rs` | All three browsers find the host | VM per browser |
| P4.3 | Extension (one source, MV3): Chrome `service_worker`, Firefox `background.scripts` + `browser_specific_settings.gecko.id`. Popup shows the layer status | new `extension/` | Loads unpacked in Chrome and Firefox | Manual + web-ext lint |
| P4.4 | "App not installed" path: detect the `lastError`, show an install page that links to GitHub Releases, and run browser-only checks | `extension/` | Works with the app uninstalled | Manual |
| P4.5 | `open_app` via a `cachelper://` deep link (`tauri-plugin-deep-link`) for mutating fixes, so all changes happen in the app with confirmation | `lib.rs`, `extension/` | The extension can't mutate anything directly | Code review + manual |
| P4.6 | Store listings. **Ask the owner first**: the Chrome Web Store fee is a paid account (VER-020) | — | Owner decision recorded | — |

---

## Phase 5: Unified guided setup (→ 0.5.0)

**Goal.** One "Check everything" path. It runs the layers in order (PC/SC service → reader → card → OS cert exposure / middleware → DoD trust → Firefox), stops at the first broken layer, and shows a single next step.

| ID | Task | Files | Acceptance | Test |
|---|---|---|---|---|
| P5.1 | `check_all` command returning ordered layer results (shared with the NMH) | `lib.rs` | One call returns every layer | Unit test with mocked layers |
| P5.2 | Stepper UI: layer list, first failure expanded, plain-English fix, a "Re-check" per step. "Not endorsed by DoD" on every screen | `index.html`, `main.js`, `styles.css` | Keyboard-accessible, readable at 480 px | Manual + axe check |
| P5.3 | Copyable diagnostic report (no PII: drop the cert subject DN and keep only the issuer and expiry) for support | `lib.rs`, `main.js` | Report copies to the clipboard | Manual |

---

## Phase 6: Signing, installers, auto-update (→ 0.9.0)

**Goal.** Signed, notarized installers on GitHub Releases, with working auto-update (ADR-006).

| ID | Task | Acceptance | Test |
|---|---|---|---|
| P6.1 | Owner: enroll in the Apple Developer Program ($99/yr) and create Developer ID Application + an App Store Connect API key | Credentials in GH secrets | — |
| P6.2 | Owner: set up Azure Artifact Signing (Basic, $9.99/mo), or buy an OV cert if not eligible (VER-019) | Signing profile in GH secrets | — |
| P6.3 | `tauri-action` release workflow: Windows x64 (+ ARM64 if P0 shows it builds) NSIS, macOS arm64 + x64 DMG (or universal), signed and notarized | Artifacts pass `signtool verify /pa` and `spctl -a -vv` | CI + a manual install on clean VMs |
| P6.4 | Updater: `tauri signer generate`, store the private key + password in GH secrets **and** an offline backup, add `pubkey` + `endpoints` (GitHub `latest.json`) and `createUpdaterArtifacts` | 0.9.0 updates to 0.9.1 automatically | VM test |
| P6.5 | Installer registers the NMH manifests (P4.2) and uninstall removes them | Clean uninstall leaves no registry keys or files | VM |

---

## Phase 7: Beta on real hardware (→ 1.0.0)

| ID | Task | Acceptance |
|---|---|---|
| P7.1 | 🔧 Owner runs every 🔧 cell in [TEST-MATRIX.md](TEST-MATRIX.md) | Results recorded in the matrix |
| P7.2 | Small closed beta (5–10 service members, different readers) using GitHub Issues templates | No open P1 bugs |
| P7.3 | Security review: elevation paths, NMH input handling, CSP, updater keys | Findings fixed |
| P7.4 | Tag `v1.0.0` and publish | Signed release live |

---

## Top risks

1. **The Windows inbox driver may not expose the certificate DoD sites need** (VER-011). That would make third-party middleware (ActivClient is licensed and not freely available) necessary for some users. Mitigation: OpenSC guidance, and an honest "needs middleware" state.
2. **macOS admin-domain trust changes may still be refused** without MDM on newer macOS (VER-021). Mitigation: the Keychain Access hand-off fallback.
3. **Losing the updater signing key** strands every installed user. Mitigation: an offline backup step in P6.4.
4. **Wrong trust scope**: roots, intermediates or cross-certs in the wrong store (VER-012) undermine trust or break chains. Mitigation: Rust-side classification, the preview, and an InstallRoot diff in P1.7.
5. **The scaffold has never compiled** and macOS code is unchecked. API drift could be wider than the Windows log shows. Mitigation: P0.7 on the owner's Mac and CI on `macos-latest`.

## Hardware-only verification items (owner)

- VER-010: macOS CryptoTokenKit + CAC
- VER-011: Windows inbox minidriver + CAC
- VER-021 / VER-005: macOS trust install as the user
- VER-003: ActivClient / SafeNet / PKard paths (licensed installers)
- Every 🔧 cell in TEST-MATRIX.md

---

## `cargo check` log

Run on 2026-10-21 on Windows 11 x64 with rustc 1.99.1 / cargo 1.99.1 and VS 2022 Build Tools 17.14.

Resolved versions: `tauri 2.12.5`, `pcsc 2.9.0`, `libloading 0.8.9`, `open 5.4.4`.

The scaffold in the repo was **not** modified. Errors 2 and 3 come from a scratch copy that applied only the workaround for the error before them, so we could see what lies behind each one.

| # | Target | Error (verbatim) | Task |
|---|---|---|---|
| 1 | x86_64-pc-windows-msvc | `error: failed to parse manifest at …\src-tauri\Cargo.toml` / `Caused by: can't find library 'cac_helper_lib', rename file to 'src/lib.rs' or specify lib.path` | P0.1 |
| 2 | x86_64-pc-windows-msvc (scratch copy, `[lib]` removed) | `failed to run custom build command for 'cac-helper v0.1.0'` … `` `…\src-tauri\icons/icon.ico` not found; required for generating a Windows Resource file during tauri-build `` | P0.2 |
| 3 | x86_64-pc-windows-msvc (scratch copy, `[lib]` removed + placeholder `icons/icon.ico`) | **No errors, no warnings.** `Finished 'dev' profile … in 25.38s` | — |
| 4 | aarch64-apple-darwin, cross-checked from Windows | `failed to run custom build command for 'objc2-exception-helper v0.1.1'` … `failed to find tool "cc": program not found`. This is a host limit, not a scaffold bug: Apple clang/SDK is needed | P0.7 (owner's Mac) |

**What this means.** On Windows the Rust code in all four modules already compiles against current crate versions. The pcsc, libloading and Tauri API usage is fine (VER-006 is verified for Windows). Phase 0 on Windows is just P0.1 + P0.2, then the **runtime** fixes P0.4 and P0.5, which `cargo check` can't catch. The `cfg(target_os = "macos")` code is still uncompiled.
