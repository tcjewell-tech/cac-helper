# Verification Register

One row per assumption that isn't verified yet.

**Status** values:
- `open`: not checked
- `partial`: some evidence, not conclusive
- `verified`: confirmed against the source listed
- `refuted`: the assumption is wrong, and a fix is planned

🔧 means it needs the owner's physical hardware.

Retrieval date for every web source: **2026-10-21**, except rows marked **P1 2026-11-15**, which were re-checked in the Phase 1 sprint. Their evidence is in [Phase 1 evidence](#phase-1-evidence-2026-11-15) below.

| ID | Claim | Where in code | How to verify | Source | Status |
|---|---|---|---|---|---|
| VER-001 | The current DoD roots are "DoD Root CA 3, 4, 5, 6" | `cert_store.rs` `KNOWN_DOD_ROOTS` | Compare against the current DISA PKCS#7 bundle README and the Windows 11 STIG. Pin by thumbprint | **P1 2026-11-15.** Signed DISA bundle v5.14 ([download](https://dl.dod.cyber.mil/wp-content/uploads/pki-pke/zip/unclass-certificates_pkcs7_DoD.zip), signature verified, see E1). Win 11 STIG V-253427 ([v2r8](https://www.stigviewer.com/stigs/microsoft_windows_11_v2r8/2026-05-19/finding/V-253427)) lists 3, 5, 6 | **verified** for Root CA 3/5/6 (thumbprints in E1, now in `KNOWN_DOD_ROOTS`). Root CA 4 is dropped from the *required* list. **Note:** Root CA 4 is still shipped in the bundle (no subordinate CAs, expires 2032), so P2.3 must not flag it as out of place |
| VER-002 | The DISA cert page is `https://public.cyber.mil/pki-pke/tools-configuration-files/` | `cert_store.rs` `DOD_CERT_PAGE_URL` | Fetch it and follow redirects. Confirm the bundle and InstallRoot are listed | **P1 2026-11-15.** [www.cyber.mil/pki-pke/tools-configuration-files](https://www.cyber.mil/pki-pke/tools-configuration-files/), rendered by a headless browser, lists "PKCS#7 for DoD PKI Only - Version 5.14" (2025-03-03) and four InstallRoot 5.6 Windows installers. The STIG fix text cites the same path | **verified**. Constant updated to the `www.cyber.mil` URL. A 10-second look in Tom's own browser is still worthwhile |
| VER-003 | Middleware install paths (OpenSC, ActivClient `acpkcs211.dll`, SafeNet `eTPKCS11.dll`, PKard framework, Homebrew paths) | `middleware.rs` `known_candidates()` | Vendor docs, then confirm on a real install | **P1 2026-11-15**, vendor docs only, see E2 | **partial**. OpenSC (Windows and macOS pkg), ActivClient and SafeNet paths match vendor docs. The PKard path and the Homebrew path are not confirmed. Nothing was installed (no VM here). OpenSC on Windows gets confirmed on a real install in step 4 of the [hardware checklist](PHASE1-HARDWARE-CHECKLIST.md) |
| VER-004 | `pkcs11.txt` lists modules as `library=<path>` lines, so a substring match works | `firefox_nss.rs` `module_registered_in_profile` | Read a real profile's `pkcs11.txt` (owner approval needed to read a profile). Compare with NSS `pk11pars` format docs | NSS source | open. Lower stakes after ADR-001: now used only to report leftovers |
| VER-005 | macOS nested shell + AppleScript escaping is safe for paths with spaces, quotes and non-ASCII | `cert_store.rs` `shell_single_quote` / `applescript_escape` | Unit-test the escaping functions. On a Mac, install a test cert from `~/Test Dir/it's ünïcode/ca.cer` | — | **superseded** by ADR-004a (osascript removed). Re-test the new argv-based path on the Mac 🔧 |
| VER-006 | The `pcsc` 2.x API (`Context::establish`, `list_readers`, `ReaderState::new`, `get_status_change`, `Error` PartialEq) matches the scaffold | `reader.rs` | `cargo check` and docs.rs for the resolved version | `cargo check` on 2026-10-21 against pcsc 2.9.0: clean (PLAN.md log) | **verified on Windows**. macOS cfg code is unchecked (P0.7 🔧) |
| VER-007 | A cancelled UAC prompt makes PowerShell exit 1223 | `cert_store.rs` `install_one` (Windows) | On Windows, run the exact `Start-Process … -Verb RunAs -PassThru` command, cancel UAC, and record the exit code and stderr. Expected: `Start-Process` throws "The operation was canceled by the user", so the exit code is 1, not 1223 | Microsoft `Start-Process` docs | open. **P1 2026-11-15:** safe on any Windows PC (it changes no trust store), but a person has to click "No" on the prompt, so it is step 5 of the [hardware checklist](PHASE1-HARDWARE-CHECKLIST.md). Lower stakes after ADR-007 (only P3.5 uses elevation) |
| VER-008 | `certutil -addstore` accepts a `.p7b` directly | `cert_store.rs` | `certutil -dump bundle.p7b` (read-only), then `-addstore` into a throwaway VM | certutil reference doesn't say | open. Low stakes: ADR-003 splits the bundle in Rust anyway. Moot on Windows after ADR-007 (no installer) |
| VER-009 | `security add-trusted-cert` accepts a `.p7b` | `cert_store.rs` (macOS) | Run it on a Mac with the DISA bundle | `man security` says it takes one cert | open. Low stakes: ADR-003 |
| VER-010 | macOS 13+ reads a CAC with no third-party software (CryptoTokenKit PIV), and Safari/Chrome/Firefox can use it | `middleware.rs` (new `OsNative`) | 🔧 Insert a CAC into an Apple Silicon Mac and run `sc_auth identities`, `security list-smartcards`, then a CAC login in each browser | Apple deployment guide "Intro to smart card integration" | partial (vendor doc yes, CAC-specific hardware test pending) |
| VER-011 | Windows 11's inbox PIV minidriver exposes the CAC certificates DoD sites need, with no ActivClient | `middleware.rs` (new `OsNative`) | 🔧 Clean Windows 11 + CAC: run `certutil -scinfo` and list the certs, then try a CAC-protected DoD site in Edge and Firefox | MS inbox minidriver doc (PIV from Win7 SP1). militarycac.com says only PIV/Email certs are exposed | partial |
| VER-012 | Intermediates belong in Windows `CA`, roots in `ROOT`, interop cross-certs in `Disallowed` | `cert_store.rs` (Phase 2 rewrite) | Compare with what DISA InstallRoot 5.6 does in a VM (the stores before and after) | Win 11 STIG V-253427/V-253429 | partial. **P1 2026-11-15: needs a VM.** InstallRoot changes the system trust stores, so it was not run on the dev PC, and this PC has no Windows Sandbox. After ADR-007 this informs the Windows **check** (P2.3) only |
| VER-013 | DISA has no current InstallRoot for macOS | docs / UI copy | Check the cyber.mil tools page | **P1 2026-11-15.** The [tools page](https://www.cyber.mil/pki-pke/tools-configuration-files/) lists only Windows InstallRoot installers. [Getting Started](https://www.cyber.mil/pki-pke/end-users/getting-started) says Mac and Linux users "can import certificates from the PKCS 7 bundle" | **verified** |
| VER-014 | Edge NMH registry key and macOS path | ADR-005 | Microsoft Edge native messaging docs | **P1 2026-11-15.** [Edge native messaging](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging) | **verified**. Windows: `HKCU\SOFTWARE\Microsoft\Edge\NativeMessagingHosts\<name>` (default value = manifest path). macOS per-user, Stable channel: `~/Library/Application Support/Microsoft Edge/NativeMessagingHosts/<name>.json`. E3 has two details that affect P4.2 |
| VER-015 | An MV3 service worker can do one-shot `sendNativeMessage` without lifetime problems | ADR-005 | Chrome service worker lifecycle docs, plus a prototype | **P1 2026-11-15.** [Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging), [service worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle) | **partial**. The docs confirm the API works from a service worker, and that each call starts a fresh host process whose first message is the reply. They don't say a pending call keeps the worker alive, so the host should answer well inside 30 s. The prototype is still due in P4.1/P4.3. See E3 |
| VER-016 | Firefox 120 made OS root import (`security.enterprise_roots.enabled`) default true, so ESR 115 is still false | ADR-001 | Firefox 120 release notes on mozilla.org | **P1 2026-11-15.** [Firefox 120.0 release notes](https://www.mozilla.org/en-US/firefox/120.0/releasenotes/): "now imports user-added TLS trust anchors … from the operating system root store … enabled by default on Windows, macOS, and Android" | **verified** for the Firefox 120 default. "ESR 115 is false" follows from that (115 is older than 120) but was not checked in ESR 115 itself (test matrix row 12) |
| VER-017 | A public DoD-PKI test URL exists that the extension can probe for a client-cert prompt | ADR-005 | Find a DISA-published test page | **P1 2026-11-15.** DISA's [Getting Started](https://www.cyber.mil/pki-pke/end-users/getting-started) page and its Mac page name no test page | open. **No dedicated official test page was found.** One candidate: the "Login with DoD CAC" sign-in on [cyber.mil](https://www.cyber.mil/) itself, which DISA runs and the app already links to. It needs Tom's pick and a manual pass with a CAC 🔧 (checklist steps 1 and 2). If none is confirmed, P5.4 uses the "Try the site you need" fallback (ADR-007 #6) |
| VER-018 | Tauri v2 builds a universal macOS binary with `--target universal-apple-darwin` | ADR-006 | Tauri CLI docs plus a CI run | **P1 2026-11-15.** [Tauri v2 App Store guide](https://v2.tauri.app/distribute/app-store/): `tauri build --bundles app --target universal-apple-darwin` | **verified** in the docs. The CI run happens in P6.3 |
| VER-019 | OV cert price range, and whether individuals are eligible for Azure Artifact Signing | ADR-006 | Artifact Signing FAQ (aka.ms/artifactsigningfaq). CA price pages | Azure pricing page ($9.99/mo Basic) | partial |
| VER-020 | Chrome Web Store developer fee is $5 one-time | ADR-006 | Chrome Web Store developer docs | — | open. Owner approved the fee on 2026-10-29 (ADR-007) |
| VER-021 | The `security add-trusted-cert -d` run as a normal user shows the admin auth dialog and succeeds on macOS 13–26 | ADR-004a | 🔧 Run it on the owner's Mac with a test CA, then remove it | Apple dev forums 686068, mkcert #415 | open |
| VER-022 | A 64-bit app reports 32-bit-only middleware as "invalid" rather than "wrong architecture" | `middleware.rs` `validate_pkcs11` | Read the PE header machine field before `LoadLibrary`, and test with OpenSC x86 in a VM | — | **refuted** by code reading: the scaffold can't tell them apart. Fix in Phase 2 |
| VER-023 | The Windows trust check (`certutil -store Root` substring on CN) is locale-independent and sees the CurrentUser Root store | `cert_store.rs` `root_present` (Windows) | Run it on a non-English Windows VM. Note that it checks only LocalMachine. Phase 2 replaces it with thumbprint lookup through CryptoAPI or `certutil -store -v` | — | open |
| VER-024 | `tauri.conf.json` identifier `helper.cac.app` is acceptable | `tauri.conf.json` | `cargo tauri build` warns when an identifier ends in `.app` | Tauri config docs | open (see Phase 0) |
| VER-025 | Any reader with a card present means a CAC is present | `reader.rs` `detect_readers` | Run `cargo tauri dev` on a laptop with a cellular modem and no smart card reader | Phase 0 run on 2026-10-21: "Microsoft UICC ISO Reader 00c87406 0: card present" → "Card readable" | **refuted**. Fix in P2.1 |
| VER-026 | Legacy CAC applet AIDs are CCC `A0 00 00 01 16 DB 00` and PKI applets `A0 00 00 00 79 01 0x`, and a CAC without a PIV applet still answers them | `reader.rs` (P2.1 `CacWithoutPiv`) | Check the spec, then send SELECT APDUs to a real CAC 🔧 (P1.10) | **P1 2026-11-15.** [DoD Implementation Guide for CAC Next Generation v2.6](https://www.cac.mil/Portals/53/Documents/CAC_NG_Implementation_Guide_v2.6.pdf) (Nov 2006), §4.3, §5.1.3, Table 2. Cross-check: [OpenSC `card-cac.c`](https://github.com/OpenSC/OpenSC/blob/master/src/libopensc/card-cac.c) | **partial**. The CCC AID is confirmed in both sources. The PKI applet AIDs are consistent with the guide but are not quoted in full there. The hardware test is checklist step 4 🔧. See E4, which also suggests a simpler probe for P2.1 |

---

## Phase 1 evidence (2026-11-15)

Everything below was gathered on 2026-11-15 without a CAC and without changing any trust store. Downloaded files stayed in a scratch folder and are **not** committed (the plan says not to commit DoD certificates).

### E1: DISA bundle and root thumbprints (P1.1, VER-001/002)

- **Bundle:** "PKI CA Certificate Bundles: PKCS#7 for DoD PKI Only - Version 5.14", upload date 2025-03-03 on the DISA tools page.
- **File:** `unclass-certificates_pkcs7_DoD.zip`, 121,215 bytes, SHA-256 `32595adbe752df5823cedd2c6a4f206c07fcc3c1520fb831015204e9fbb75711`. It was downloaded over HTTPS from `dl.dod.cyber.mil`, with the server certificate validated by Windows.
- **Signature check:** done the way the bundle's own README describes. `openssl smime -verify -in Certificates_PKCS7_v5_14_DoD.sha256 -inform DER -CAfile DoD_PKE_CA_chain.pem` printed `Verification successful`, and all 7 file hashes in the signed list matched (`sha256sum -c` reported `OK` for each). The signer chains through DOD EMAIL CA-73 to DoD Root CA 6.
- **Contents:** 49 certificates. Four are self-signed roots:

| Root | SHA-1 thumbprint | SHA-256 thumbprint | Valid until | Subordinate CAs in the bundle |
|---|---|---|---|---|
| DoD Root CA 3 | `D73CA91102A2204A36459ED32213B467D7CE97FB` | `B107B33F453E5510F68E513110C6F6944BACC263DF0137F821C1B3C2F8F863D2` | 2029-12-30 | 17 |
| DoD Root CA 5 | `4ECB5CC3095670454DA1CBD410FC921F46B8564B` | `1F4EDE9DC2A241F6521BF518424ACD49EBE84420E69DAF5BAC57AF1F8EE294A9` | 2041-06-14 | 7 |
| DoD Root CA 6 | `D37ECF61C0B4ED88681EF3630C4E2FC787B37AEF` | `2A5E41ACA93DF7CC496D2369B7A0A037045D502F1ABAAF76975FC07C6660CF93` | 2053-01-24 | 21 |
| DoD Root CA 4 | `B8269F25DBD937ECAFD4C35A9838571723F2D026` | `559A5189452B13F8233F0022363C06F26E3C517C1D4B77445035959DF3244F74` | 2032-07-25 | **0** |

- The Root CA 6 SHA-1 also matches the value printed in DISA's README. The Root CA 3 and 6 values match the earlier unconfirmed candidates.
- **Root CA 4:** it is still shipped, but nothing in the bundle chains to it, and the STIG's list is 3, 5, 6. So it is not *required*. It is also not *retired or out of place*. PLAN P2.3 and ADR-007 #8 use "retired roots such as Root CA 4" as their example of an out-of-place cert. That example is wrong and should be changed before P2.3 is built, or the app will warn on every machine where InstallRoot has run.
- **Expired intermediates:** the bundle still carries DOD ID CA-59, DOD EMAIL CA-59, DOD SW CA-60 and DOD SW CA-61, which expired in March and April 2025. P2.3's "report expired" should cover roots only, so the app doesn't warn about certificates DISA itself ships.
- **Code:** `KNOWN_DOD_ROOTS` now holds name + SHA-1 for Root CA 3/5/6, and the check matches on the thumbprint. `cargo check` is clean on Windows. The macOS branch (`security find-certificate -a -c <name> -Z`) is uncompiled and untested, like the rest of the macOS code (P0.7).
- **This dev PC** has none of the four roots in LocalMachine `Root` (`certutil -store Root <thumbprint>` answers "Object was not found"), so the app correctly reports `NotInstalled` here.

### E2: Middleware paths from vendor docs (P1.2, VER-003)

| Middleware | Path in `middleware.rs` | Vendor source | Result |
|---|---|---|---|
| OpenSC, Windows 64-bit | `C:\Program Files\OpenSC Project\OpenSC\pkcs11\opensc-pkcs11.dll` | [OpenSC wiki: Windows Quick Start](https://github.com/OpenSC/OpenSC/wiki/Windows-Quick-Start) | matches |
| OpenSC, 32-bit on 64-bit Windows | `C:\Program Files (x86)\OpenSC Project\OpenSC\pkcs11\opensc-pkcs11.dll` | [OpenSC wiki: Firefox, step by step](https://github.com/OpenSC/OpenSC/wiki/Installing-OpenSC-PKCS11-Module-in-Firefox,-Step-by-Step) | matches |
| OpenSC, macOS pkg installer | `/Library/OpenSC/lib/opensc-pkcs11.so` | [OpenSC wiki: macOS Quick Start](https://github.com/OpenSC/OpenSC/wiki/macOS-Quick-Start) | matches |
| OpenSC, macOS `/usr/local/lib` | `/usr/local/lib/opensc-pkcs11.so` | [OpenSC issue 1008](https://github.com/OpenSC/OpenSC/issues/1008) says it is a link to the pkg path | plausible (secondary source) |
| OpenSC, Homebrew on Apple Silicon | `/opt/homebrew/lib/opensc-pkcs11.so` | not checked | **open** |
| ActivClient | `C:\Program Files\HID Global\ActivClient\acpkcs211.dll` (and the x86 folder) | [HID ActivClient 7.4.1: Files and Processes](https://docs.hidglobal.com/activid-activclient-v7.4.1/activid-activclient/getting-started/activclient-files-and-processes.htm). Default folder `%ProgramFiles%\HID Global\ActivClient`, with `acpkcs211.dll` in `%INSTALLDIR%` and `%INSTALLDIRX86%` | matches |
| Thales SafeNet | `C:\Windows\System32\eTPKCS11.dll` | [Thales SAC integration guide](https://cpl.thalesgroup.com/sites/default/files/integration-guide/007-012954-001_SAC_IntegrationGuide_IBMNotes_CBA_RevA.pdf) gives `C:\Windows\System32\eTpkcs11.dll` | matches (Windows paths ignore case) |
| PKard | `/Library/Frameworks/PKard.framework/PKard` | [Thursby downloads](https://downloads.thursby.com/) says PKard for Mac "is no longer needed" from macOS Catalina on | **path not confirmed.** The product is obsolete on every macOS the app supports, so consider dropping this candidate in P2.2 |

### E3: Browser and Tauri facts (P1.8, VER-014/015/016/018)

- **Edge fallback (affects P4.2).** On Windows, if Edge finds no key under `Microsoft\Edge\NativeMessagingHosts`, it falls back to the Chromium and then the Chrome registry locations. It reads only the first manifest it finds, so that manifest must list the extension IDs for both stores in `allowed_origins`.
- **Edge on macOS.** Other channels use `Microsoft Edge Beta`, `Dev` or `Canary` in the path.
- **Message size.** Chrome says browser→host is at most 64 MiB. Edge's page says 4 GB. Host→browser is 1 MB in both. ADR-005's numbers are right for Chrome.
- **Service worker (Chrome).** The worker stops after 30 s idle, and events or extension API calls reset that timer. A single request may not run longer than 5 minutes. The docs state that `connectNative()` keeps the worker alive. For `sendNativeMessage()` they say only that a new host process starts per message and that the first message back is the response.
- **Firefox 120.** The release note says "user-added" trust anchors. DoD roots installed by InstallRoot or by hand are user-added, so this is the setting the app cares about.

### E4: CAC applet AIDs (P1.10, VER-026)

Source: *DoD Implementation Guide for CAC Next Generation (NG)*, version 2.6, November 2006, published on cac.mil. This is the DoD guide. NIST IR 6887 itself was not opened.

| What | Value | Where |
|---|---|---|
| CCC (Card Capability Container) AID | `A0 00 00 01 16 DB 00` | Guide §5.1.3. Same in OpenSC `cac_CCC_Path` |
| RID, DoD CAC data model | `A0 00 00 00 79` | Guide §4.3. Same in OpenSC |
| RID, PIV transitional (GSC-IS 2.1) | `A0 00 00 01 16` | Guide §4.3 |
| RID, PIV end-point | `A0 00 00 03 08` | Guide §4.3. OpenSC `card-piv.c` uses the full AID `A0 00 00 03 08 00 00 10 00` |
| PKI ID / Signature / Encryption applets | application IDs `0100` / `0101` / `0102` under RID `A0 00 00 00 79`, which gives `A0 00 00 00 79 01 00`, `…01 01`, `…01 02` | Guide Table 2 lists the IDs. The 7-byte AIDs are put together here from RID + ID. The guide does not print them in full |
| CAC ACA (access control applet) | `A0 00 00 00 79 10 00` | OpenSC `cac_ACA_Path` only |

- The guide was read through an automated PDF extraction, because cac.mil refused a direct download. The CCC value and the RIDs were cross-checked against OpenSC's source. The Table 2 IDs still need a human look at the PDF.
- **Suggestion for P2.1.** OpenSC recognises a CAC by selecting the CCC and the ACA, not the PKI applets. Probing the CCC AID alone is enough to say "this is a CAC", and it is the best-attested value.
- **Still open 🔧.** Whether a real CAC answers these, and whether any CAC in circulation lacks the PIV applet (checklist step 4).
