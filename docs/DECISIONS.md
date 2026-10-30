# Architecture Decision Records

Every source below was retrieved on **2026-10-21** unless a row says otherwise. Claims we could not confirm are listed in [VERIFICATION.md](VERIFICATION.md) instead of being stated as fact here.

---

## ADR-001: Firefox integration without `modutil`

**Status:** Accepted (owner approved the module 4 redesign on 2026-10-21)

**Context.** Module 4 registers the PKCS#11 middleware into each Firefox profile using `modutil`. That tool comes with NSS tools, not with Firefox, so the app would have to bundle third-party binaries. Firefox also has to be closed while it runs.

**Options**

| | (a) Bundle `modutil` + NSS libs | (b) Enterprise policy (`SecurityDevices`, `Certificates.ImportEnterpriseRoots`) | (c) Built-in OS client-cert module (`osclientcerts`) + OS trust |
|---|---|---|---|
| Firefox must be closed | Yes, the NSS DB is locked | No for the write, but a restart is needed for it to apply | No |
| Privilege | User (the profile dir) | Admin (Windows: HKLM policy key or `distribution/policies.json` in Program Files; macOS: a plist in `/Library/Preferences` or inside `Firefox.app`) | None in Firefox. Cert trust comes from module 3 |
| Scope | Per profile | Machine-wide, every profile | Machine or user (OS store) |
| ESR vs. release | Works on all versions | `SecurityDevices` since Fx 64 / ESR 60.4, `Add`/`Delete` since Fx 114 / ESR 102.12. `ImportEnterpriseRoots` since Fx 60, macOS since Fx 63 / ESR 68 | `osclientcerts` on by default since Fx 90. OS root import on by default since Fx 120, so ESR 115 needs the pref or policy |
| Undo | `modutil -delete` per profile | Remove the policy key/file, or `SecurityDevices.Delete` | Nothing to undo |
| Bundles 3rd-party binaries | **Yes** (a stop-and-ask trigger) | No | No |

**Evidence.**
- Current Firefox source `modules/libpref/init/StaticPrefList.yaml` sets `security.osclientcerts.autoload: true` ("allows Firefox to use client certificates stored in OS certificate storage. Currently only available for Windows and macOS") and `security.enterprise_roots.enabled: true`.
- `security/manager/ssl/EnterpriseRoots.cpp` reads the Windows `ROOT` store (as trust anchors) and the `CA` store (as intermediates), across the LocalMachine, CurrentUser, GroupPolicy and Enterprise locations. On macOS it reads keychain trust settings.

**Decision: (c).** Module 4 becomes a **read-only Firefox diagnostic**. It checks:
1. The Firefox version (release vs. ESR, and whether it is at least 120).
2. That `security.osclientcerts.autoload` and `security.enterprise_roots.enabled` are not overridden to false in `prefs.js`/`user.js` or by policy.
3. Leftover or conflicting PKCS#11 entries in `pkcs11.txt`. These are reported, never edited.

Only for edge cases does it **offer** policy (b) as an opt-in, confirmed fix:
- Firefox ESR older than 120
- OpenSC-only setups where the OS can't expose the card

`modutil` registration is removed from the code.

**Consequences.**
- No bundled NSS binaries.
- No need to close Firefox.
- Module 4 depends on module 3 (the DoD roots must be in the OS store) and on module 2 (the OS must expose the card's certificates).
- `register_nss` and the `modutil` call are deleted in Phase 3.

**Sources.**
- https://raw.githubusercontent.com/mozilla-firefox/firefox/main/modules/libpref/init/StaticPrefList.yaml
- https://raw.githubusercontent.com/mozilla-firefox/firefox/main/security/manager/ssl/EnterpriseRoots.cpp
- https://firefox-admin-docs.mozilla.org/reference/policies/securitydevices/
- https://mozilla.github.io/policy-templates/ (says it is not current; points to firefox-admin-docs)
- https://blog.mozilla.org/security/2021/07/28/making-client-certificates-available-by-default-in-firefox-90/ (returned 403 to our fetcher; content confirmed through search summaries)
- Firefox 120 OS-trust-anchor default: secondary reports only. **Unverified** (VER-016).

---

## ADR-002: OS-native CAC support; "no third-party middleware" is a valid state

**Status:** Accepted, pending hardware verification (VER-010, VER-011)

**Context.** Module 2 treats "no OpenSC/ActivClient/SafeNet/PKard found" as `NotFound`, which is a failure.

**Evidence.**
- **macOS:** Apple's deployment guide says macOS 10.15+ has native support for PIV smart cards and USB CCID class-compliant readers through CryptoTokenKit, "without any additional software". It requires certificates in PIV slots 9a and 9d. The CAC carries a PIV applet.
- **Windows:** Microsoft's "Windows inbox smart card minidriver" doc says the inbox class minidriver supports PIV-compliant cards from Windows 7 SP1. It detects PIV with a SELECT of the PIV AID.
- Community sources (militarycac.com, dated) report that the inbox driver exposes only the PIV-applet certificates (PIV Auth, and email on Windows 10). The legacy DoD "Identity" certificate on the CAC applet may not be exposed. **Unverified on Windows 11** (VER-011).

**Decision.** The module 2 outcome model becomes:
- `OsNative`: no third-party module, and the OS sees the card's certificates. **This is a pass.**
- `ThirdPartyValid { name, path }`
- `ThirdPartyInvalid { name, path, reason }`, where `reason` ∈ {`LoadFailed`, `ArchMismatch`, `NoFunctionList`, `FunctionListError`}
- `NothingUsable`

To detect `OsNative`, enumerate the OS's smart card certificates:
- **Windows:** CurrentUser `MY` certificates whose key provider is `Microsoft Smart Card Key Storage Provider`, found through CNG or `certutil -scinfo`.
- **macOS:** `security list-smartcards` / `system_profiler SPSmartCardsDataType`, plus `sc_auth identities`.

**Consequences.**
- The "Middleware status" card can be green without any install.
- Third-party middleware remains a fallback for setups where the inbox driver doesn't expose the certificate the site needs.

**Sources.**
- https://support.apple.com/guide/deployment/intro-to-smart-card-integration-depd0b888248/web
- https://support.apple.com/guide/deployment/supported-smart-card-functions-on-mac-depc47f60521/web
- https://github.com/MicrosoftDocs/windows-driver-docs/blob/staging/windows-driver-docs-pr/smartcard/windows-inbox-smart-card-minidriver.md
- https://www.militarycac.com/PIV.htm (secondary, dated; used only to flag the risk)

---

## ADR-003: Handling the DoD PKI `.p7b` bundle

**Status:** Accepted

**Context.** DISA ships the "PKI CA Certificate Bundles: PKCS#7 for DoD PKI Only" zip (version 5.x). It contains `.p7b` files that mix roots and intermediates.

**Evidence.**
- Microsoft's `certutil` reference documents `-addstore CertificateStoreName InFile` with "the certificate or CRL file". It says **nothing** about PKCS#7 bundles.
- Apple's `security add-trusted-cert` takes a single certificate file.
- Even if `certutil` accepted a `.p7b`, putting the whole bundle into `ROOT` would make every intermediate a trust anchor. That is wrong (see ADR-004).

**Decision.** The app parses the bundle itself in Rust and installs certificate by certificate:
- Use the RustCrypto `cms` + `x509-cert` crates (pure Rust, no OpenSSL) to parse `SignedData.certificates`.
- Classify each certificate as root (self-signed, with CA basicConstraints), intermediate, or cross-cert.
- Show the user the list (subject, issuer, SHA-256 fingerprint, expiry) before any change.
- Accept `.zip` too, so the user can pick the DISA zip directly. Use the `zip` crate in read-only mode.

**Consequences.**
- Two to three new dependencies (`cms`, `x509-cert`, `zip`), all pure Rust.
- `certutil`'s handling of `.p7b` no longer matters (VER-008 is kept for information).
- `certutil -dump <file>` remains a useful read-only test oracle.

**Sources.**
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/certutil
- `man security` (add-trusted-cert)
- https://cs.nps.edu/web/technology/dod-certificates (secondary; gives the bundle naming)

---

## ADR-004: Trust scope (where each certificate goes)

**Status:** Accepted, pending verification (VER-001, VER-012)

**Decision**

| Certificate type | Windows | macOS |
|---|---|---|
| DoD Root CA (self-signed) | LocalMachine `ROOT` | System keychain, admin-domain trust `trustRoot` |
| DoD intermediate / issuing CAs | LocalMachine `CA` (Intermediate Certification Authorities). **Not** `ROOT` | System keychain, **no** trust setting (`security add-certificates`) |
| DoD Interoperability Root CA cross-certs | **Do not install.** The STIG puts them in `Disallowed` on unclassified systems; recommend InstallRoot for that | Do not install |

**Evidence.**
- The Windows 11 STIG (V-253427 / V-253429 lineage, mirrored on stigviewer.com) lists **DoD Root CA 3, 5 and 6** as the valid roots and **no longer lists Root CA 4**.
- The same STIG says the Interoperability cross-certificates belong in Untrusted Certificates.
- Firefox reads Windows `ROOT` as trust anchors and `CA` as intermediates (ADR-001), so this split works for Firefox too.

**InstallRoot.**
- **Windows:** DISA's own InstallRoot (5.6, with a "Non-Administrator" installer) is the **recommended primary path**. The app checks the result, and its own installer is a fallback for users who can't run InstallRoot.
- **macOS:** DISA has no current InstallRoot for macOS (community source, VER-013), so the app's guided install is the primary path there.

**Consequences.**
- `KNOWN_DOD_ROOTS` must drop Root CA 4, or every correct install reports `PartiallyInstalled`.
- The check should match on **SHA-1/SHA-256 thumbprint**, not CN substring. Candidate thumbprints are in VER-001.
- Detecting expiry is part of the check.

**Sources.**
- https://www.stigviewer.com/stigs/microsoft_windows_11_v2r8/2026-05-19/finding/V-253427
- https://stigaview.com/products/win11/v1r5/WN11-PK-000005
- https://www.stigviewer.com/stigs/microsoft_windows_11/2026-05-19/finding/V-253429
- https://www.cyber.mil/pki-pke/tools-configuration-files/ (redirect target of the scaffold URL; the page renders client-side and its content wasn't readable by our fetcher)

### ADR-004a: macOS install mechanism

**Status:** Accepted (owner approved on 2026-10-21), pending hardware verification (VER-005)

**Context.** The scaffold runs `security add-trusted-cert -d` as root through `osascript … with administrator privileges`. Since macOS 11, changing admin-domain trust settings needs an interactive user authorization. Running as root without a UI session fails with *"The authorization was denied since no user interaction was possible"*. Apple's supported path for unattended installs is MDM.

**Decision.**
- Drop `osascript`.
- Run `security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain <cert>` **as the logged-in user**. `security` then asks Authorization Services, which shows the native admin and Touch ID dialog.
- If Phase 1 testing shows this fails, the fallback is a guided Keychain Access hand-off (`open -a "Keychain Access" <cert>` plus step-by-step instructions).

**Consequences.** The shell-in-AppleScript escaping risk (VER-005) goes away. We just pass argv with no shell.

**Sources.**
- https://developer.apple.com/forums/thread/686068
- https://github.com/FiloSottile/mkcert/issues/415
- https://developer.apple.com/forums/thread/671582

---

## ADR-005: Browser extension and Native Messaging

**Status:** Accepted

**Design**
- **Host:** a small separate binary, `cac-helper-nmh` (a second `[[bin]]` in the same Cargo workspace). It reuses the module 1–4 read-only functions. It never performs mutating actions. Instead it can ask the desktop app to open (`open` with a `cachelper://` deep link through `tauri-plugin-deep-link`), and the user confirms there.
- **Host name:** `com.cachelper.nmh`. It must be lowercase alphanumerics and dots.
- **Manifests**

| Browser | Windows | macOS (per-user) |
|---|---|---|
| Chrome | `HKCU\Software\Google\Chrome\NativeMessagingHosts\com.cachelper.nmh` (default value = full manifest path). Chrome checks the 32-bit view first | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.cachelper.nmh.json` |
| Edge | `HKCU\Software\Microsoft\Edge\NativeMessagingHosts\…` (VER-014) | `~/Library/Application Support/Microsoft Edge/NativeMessagingHosts/` (VER-014) |
| Firefox | `HKCU\Software\Mozilla\NativeMessagingHosts\com.cachelper.nmh`. Fx 64+ checks `Wow6432Node` first | `~/Library/Application Support/Mozilla/NativeMessagingHosts/com.cachelper.nmh.json` |

  - Chrome manifests use `allowed_origins: ["chrome-extension://<id>/"]`.
  - Firefox manifests use `allowed_extensions: ["cac-helper@<domain>"]`, set through `browser_specific_settings.gecko.id`.
  - Both use `type: "stdio"`.
  - Registration is per-user (HKCU / `~/Library`), so it needs **no elevation**. The installer writes it, and the app's settings screen explains it.
- **Manifest versions**
  - Chrome is MV3 only: `background.service_worker`. Use `runtime.sendNativeMessage` for one-shot checks, so we don't rely on a long-lived port keeping the service worker alive (VER-015).
  - Firefox uses MV3 with `background.scripts` (an event page). The same source can be shared if we use the `browser.*` namespace through `webextension-polyfill`, or feature-detect `chrome.*`.
- **Framing:** 32-bit native-endian length + UTF-8 JSON. Host→browser max is 1 MB. Browser→host max is 64 MiB.
- **Message schema (v1)**

```jsonc
// request
{ "v": 1, "id": "uuid", "cmd": "ping" | "check_all" | "check_reader" | "check_middleware" | "check_trust" | "check_firefox" | "open_app" }
// response
{ "v": 1, "id": "uuid", "ok": true, "app_version": "0.4.0", "result": { /* same JSON the Tauri commands return */ } }
{ "v": 1, "id": "uuid", "ok": false, "error": { "code": "UNKNOWN_CMD" | "INTERNAL", "message": "..." } }
```

  - The host checks the caller origin (argv[1]) against its allow-list.
- **App not installed:** `chrome.runtime.lastError` is set to "Specified native messaging host not found." The extension still runs browser-only checks:
  - Fetch a DoD-PKI-protected test URL to see whether a client cert prompt appears (VER-017).
  - Show a static "Install CAC Helper" page that links to GitHub Releases.

**Sources.**
- https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging
- https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_manifests

---

## ADR-006: Distribution, signing and updates

**Status:** Accepted

| Item | Decision | Approx. cost |
|---|---|---|
| macOS signing + notarization | Apple Developer Program. "Developer ID Application" cert (only the Account Holder can create it). Notarize with an App Store Connect API key (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`). Ship separate `aarch64` and `x86_64` DMGs, or a universal binary (`--target universal-apple-darwin`, VER-018) | $99/yr |
| Windows signing | **Azure Artifact Signing** (formerly Trusted Signing), Basic plan, through `bundle.windows.signCommand`. The fallback is an OV certificate from a CA, using `certificateThumbprint` + `timestampUrl`. Since 2024, EV no longer buys instant SmartScreen reputation, so don't pay for EV | $9.99/mo (Basic, 5,000 signatures). OV ≈ $200–$400/yr (VER-019). Individual eligibility for Artifact Signing is unverified (VER-019) |
| Updater | `tauri-plugin-updater`, keys from `tauri signer generate`, `plugins.updater.pubkey` in config, `bundle.createUpdaterArtifacts: true`. The private key + password live only in GitHub Actions secrets (`TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`) plus an offline backup (password manager plus a printed recovery copy). **Losing the key means installed users can't be updated** | Free |
| Hosting | GitHub Releases. Updater endpoint `https://github.com/<owner>/cac-helper/releases/latest/download/latest.json`, generated by `tauri-apps/tauri-action` | Free |
| Extension stores | Chrome Web Store one-time developer fee. Firefox AMO is free | $5 one-time (VER-020) |

The Chrome Web Store fee is a small paid account beyond the two the doc allows. It is flagged for owner approval in Phase 4.

**Sources.**
- https://v2.tauri.app/plugin/updater/
- https://v2.tauri.app/distribute/sign/windows/
- https://v2.tauri.app/distribute/sign/macos/
- https://azure.microsoft.com/en-us/pricing/details/trusted-signing/

---

## ADR-007: Owner decisions from the design review (2026-10-29)

**Status:** Accepted (owner)

**Context.** A structured question-and-answer review of the plan with the owner. Each decision below names the earlier ADR or plan task it changes.

| # | Decision | Changes |
|---|---|---|
| 1 | **"OS-native" means the OS sees the card's PIV Authentication certificate.** The app does not try to guess which certificate a given site needs. The guided flow ends with a "Try your CAC" step, and a failure there routes to middleware guidance | ADR-002 (narrows the definition of `OsNative`) |
| 2 | "Try your CAC" uses a **known official DoD CAC-protected test page** | New P1.9, P5.4 |
| 3 | The card check tries the **PIV applet, then the legacy CAC applet**. A card that answers only the CAC applet gets `CacWithoutPiv` (install middleware). A card that answers neither gets `CardNotCac` | P2.1, new P1.10 / VER-026 |
| 4 | **Windows: no cert installer.** The app checks the stores and guides the user to DISA InstallRoot. `.p7b`/`.zip` parsing and installing is macOS-only | Supersedes ADR-004's Windows fallback installer. P2.4, P2.6 |
| 5 | App identifier: **`io.github.tcjewelltech.cachelper`** (the hyphen in the username is dropped for bundle-ID safety) | P0.6 |
| 6 | If no stable official test page exists, or it stops loading, the step falls back to **"Try the site you need"**. The app checks once per launch that the test page still loads | P5.4 |
| 7 | The desktop app shows **"It worked" / "It failed"** buttons for the test. Once the extension is installed, it reports the result automatically | P5.4 |
| 8 | The Windows trust check **reports** out-of-place certs (retired roots, cross-certs in `ROOT`, CurrentUser-only roots), with the fix "Run DISA InstallRoot". The app never removes certs | P2.3 |
| 9 | The **hardware checks P1.5 (Windows + CAC) and P1.6 (Mac + CAC) run first**. Phase 2 waits for their results | Phase 1 order and exit criteria |
| 10 | The owner approves the **$5 Chrome Web Store** fee | ADR-006, P4.6, VER-020 |

**Consequences.**
- On Windows, the app runs no elevated code for certificates at all. The VER-007 (UAC cancel) and VER-008 (`certutil` + `.p7b`) findings now matter only for the opt-in Firefox policy write (P3.5).
- A third-party URL (the test page) becomes a runtime dependency, so it needs a health check and a fallback.
- If P1.5/P1.6 refute OS-native support, Phase 2's middleware guidance grows and the plan is reviewed again before Phase 2.
