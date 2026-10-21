# Test Matrix

🔧 means the cell needs **physical hardware** (a reader plus a CAC), so the owner must test it personally. Every other cell can run in CI or a VM without a card.

## Checks (columns)

| ID | Check | Pass criteria |
|---|---|---|
| C1 | Builds and launches | The signed installer installs, the app opens, and there are no console errors |
| C2 | PC/SC service layer | A stopped `SCardSvr` (Win) shows `PcscServiceUnavailable`, not "no reader" |
| C3 | Reader detection | The right outcome for no reader, reader without card, and reader with card |
| C4 | Card readable | `CardPresent`. A mute or upside-down card shows `CardUnreadable` |
| C5 | OS-native certs | The OS lists the CAC's PIV Auth cert (Win `certutil -scinfo` / macOS `sc_auth identities`). Module 2 reports `OsNative` |
| C6 | Third-party middleware | Where installed, `ThirdPartyValid` with the right path. A 32-bit-only install gives `ArchMismatch` |
| C7 | DoD trust check | Matches by thumbprint: Root CA 3/5/6 present means `Installed` |
| C8 | Cert install | `.p7b`/`.zip` picked with the file picker → preview list → one elevation prompt → roots go to Root and intermediates to CA. A cancelled prompt shows the "canceled" message |
| C9 | Firefox diagnostic | Reports the version, both prefs, and leftover PKCS#11 entries. Writes nothing |
| C10 | Firefox CAC login | 🔧 A DoD CAC-protected site prompts for the cert and logs in |
| C11 | Chrome/Edge CAC login | 🔧 Same in Chrome and Edge |
| C12 | Safari CAC login | 🔧 macOS only |
| C13 | Extension ↔ host | `ping`/`check_all` round-trip works. With the app uninstalled, the extension shows the "install" page |
| C14 | Auto-update | Version N updates to N+1 from GitHub Releases, and the signature is verified |

## Rows

OS × architecture × reader × browser. Readers:
- **SCR3310** (Identiv/SCM SCR3310v2)
- **HID** (HID Omnikey 3121)
- **Generic** (an unbranded CCID reader)
- **None** (no reader)

| # | OS | Arch | Reader | Browser | C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8 | C9 | C10 | C11 | C12 | C13 | C14 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Windows 11 24H2+ | x64 | None | Firefox, Chrome, Edge | CI | VM | VM | — | — | VM | VM | VM | VM | — | — | — | VM | VM |
| 2 | Windows 11 24H2+ | x64 | SCR3310 | Firefox | ✓ | ✓ | 🔧 | 🔧 | 🔧 | 🔧 | ✓ | ✓ | ✓ | 🔧 | — | — | ✓ | ✓ |
| 3 | Windows 11 24H2+ | x64 | SCR3310 | Chrome, Edge | ✓ | ✓ | 🔧 | 🔧 | 🔧 | 🔧 | ✓ | ✓ | — | — | 🔧 | — | ✓ | ✓ |
| 4 | Windows 11 24H2+ | x64 | HID | Firefox, Chrome | ✓ | ✓ | 🔧 | 🔧 | 🔧 | 🔧 | ✓ | ✓ | ✓ | 🔧 | 🔧 | — | ✓ | ✓ |
| 5 | Windows 11 24H2+ | x64 | Generic | Chrome | ✓ | ✓ | 🔧 | 🔧 | 🔧 | — | ✓ | ✓ | — | — | 🔧 | — | ✓ | ✓ |
| 6 | Windows 10 22H2 | x64 | SCR3310 | Firefox, Chrome | ✓ | ✓ | 🔧 | 🔧 | 🔧 | 🔧 | ✓ | ✓ | ✓ | 🔧 | 🔧 | — | ✓ | ✓ |
| 7 | Windows 11 | ARM64 | SCR3310 | Edge, Firefox | ✓ | ✓ | 🔧 | 🔧 | 🔧 | 🔧 | ✓ | ✓ | ✓ | 🔧 | 🔧 | — | ✓ | ✓ |
| 8 | macOS 15/26 | arm64 | None | Safari, Chrome, Firefox | CI | — | ✓ | — | — | ✓ | ✓ | 🔧* | ✓ | — | — | — | ✓ | ✓ |
| 9 | macOS 15/26 | arm64 | SCR3310 | Safari, Chrome, Firefox | ✓ | — | 🔧 | 🔧 | 🔧 | 🔧 | ✓ | 🔧* | ✓ | 🔧 | 🔧 | 🔧 | ✓ | ✓ |
| 10 | macOS 15/26 | arm64 | HID | Firefox | ✓ | — | 🔧 | 🔧 | 🔧 | — | ✓ | 🔧* | ✓ | 🔧 | — | — | ✓ | ✓ |
| 11 | macOS 13/14 | x64 (Intel) | SCR3310 | Safari, Firefox | ✓ | — | 🔧 | 🔧 | 🔧 | 🔧 | ✓ | 🔧* | ✓ | 🔧 | — | 🔧 | ✓ | ✓ |
| 12 | Firefox ESR 115 (any OS) | x64 | SCR3310 | Firefox ESR | ✓ | — | — | — | — | — | — | — | ✓ | 🔧 | — | — | ✓ | — |

✓ means it can be tested by anyone without a card (a VM, or a real machine with no card). — means not applicable.
\*macOS C8 is marked 🔧 because the admin-domain trust prompt (ADR-004a) must be confirmed interactively on a real logged-in Mac, even without a card.

## Owner-only (hardware) summary

- Every 🔧 cell above.
- At minimum, one Windows 11 x64 machine and one Apple Silicon Mac, each with an SCR3310 and a real CAC.
- ARM64 Windows (row 7) and Intel Mac (row 11) are best-effort. If no hardware is available, they ship marked "untested".
