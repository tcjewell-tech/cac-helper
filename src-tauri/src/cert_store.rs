use serde::Serialize;
use std::process::Command;

/// Well-known DoD PKI root CA common names to check for in the OS trust
/// store. VERIFY THIS LIST against current DISA PKI/PKE documentation
/// before shipping -- DoD periodically issues new root CAs and retires
/// old ones, and this scaffold was written from memory, not a live source.
const KNOWN_DOD_ROOTS: &[&str] = &[
    "DoD Root CA 3",
    "DoD Root CA 4",
    "DoD Root CA 5",
    "DoD Root CA 6",
];

/// Official DISA page listing current DoD PKI cert bundles and installers.
/// VERIFY THIS URL before shipping -- DISA reorganizes these pages
/// periodically and a stale link is worse than no link.
pub const DOD_CERT_PAGE_URL: &str = "https://public.cyber.mil/pki-pke/tools-configuration-files/";

#[derive(Serialize, Clone, Debug)]
pub enum TrustOutcome {
    /// None of the known DoD root CAs were found in the trust store.
    NotInstalled,
    /// At least one DoD root is present, but not all of the known set --
    /// possibly an older install or a partial InstallRoot run.
    PartiallyInstalled,
    /// All known current DoD root CAs are present and trusted.
    Installed,
}

#[derive(Serialize, Clone, Debug)]
pub struct TrustCheckResult {
    pub outcome: TrustOutcome,
    pub roots_found: Vec<String>,
}

#[cfg(target_os = "windows")]
fn root_present(cn: &str) -> bool {
    // `certutil -store Root` dumps the Windows Root store; grep for the CN.
    // This only checks presence, not trust flags/expiry -- good enough for
    // a first-pass diagnostic, not a substitute for full chain validation.
    match Command::new("certutil").args(["-store", "Root"]).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains(cn),
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn root_present(cn: &str) -> bool {
    // Checks the System keychain, which is where InstallRoot / manual
    // "System" trust installs land. Exit status 0 = found.
    Command::new("security")
        .args(["find-certificate", "-c", cn, "/Library/Keychains/System.keychain"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn root_present(_cn: &str) -> bool {
    false
}

pub fn check_trust_status() -> TrustCheckResult {
    let roots_found: Vec<String> = KNOWN_DOD_ROOTS
        .iter()
        .filter(|cn| root_present(cn))
        .map(|cn| cn.to_string())
        .collect();

    let outcome = if roots_found.is_empty() {
        TrustOutcome::NotInstalled
    } else if roots_found.len() < KNOWN_DOD_ROOTS.len() {
        TrustOutcome::PartiallyInstalled
    } else {
        TrustOutcome::Installed
    };

    TrustCheckResult { outcome, roots_found }
}

#[derive(Serialize, Clone, Debug)]
pub struct InstallReport {
    pub succeeded: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Installs user-selected, already-downloaded cert files (.cer/.crt) into
/// the OS trust store.
///
/// This is a MUTATING, security-sensitive operation. Design constraints
/// that must hold when this is wired into the UI:
///   1. The frontend must show an explicit confirmation dialog naming the
///      exact files before calling this -- never trigger it automatically.
///   2. We do NOT download or bundle cert files ourselves. The user
///      downloads the official bundle from DISA (see DOD_CERT_PAGE_URL)
///      and this function only installs files they've explicitly picked.
///   3. Both platform commands trigger their own OS-native admin prompt
///      for just this action (Windows UAC via PowerShell RunAs, macOS
///      Touch ID/password via osascript) -- the app process itself never
///      runs elevated. See install_one() per platform for the mechanism.
pub fn install_cert_files(paths: Vec<String>) -> InstallReport {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for path in paths {
        match install_one(&path) {
            Ok(_) => succeeded.push(path),
            Err(e) => failed.push((path, e)),
        }
    }

    InstallReport { succeeded, failed }
}

#[cfg(target_os = "windows")]
fn install_one(path: &str) -> Result<(), String> {
    // -addstore ROOT requires an elevated (admin) process. Rather than
    // running the whole app elevated -- bad UX for modules 1/2/4, which
    // don't need it -- we launch just certutil elevated via PowerShell's
    // Start-Process -Verb RunAs. That triggers a single UAC prompt scoped
    // to this one action.
    //
    // -PassThru + exit $p.ExitCode is what lets us see certutil's real
    // exit code back on the Rust side instead of only knowing whether
    // PowerShell itself launched successfully.
    let escaped_path = path.replace('\'', "''");
    let ps_command = format!(
        "$p = Start-Process certutil -ArgumentList @('-addstore','-f','ROOT','{}') -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode",
        escaped_path
    );

    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_command])
        .output()
        .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        // Exit code 1223 (ERROR_CANCELLED) is what Windows returns when
        // the user dismisses the UAC prompt -- worth surfacing distinctly
        // since it's a user choice, not a technical failure.
        match out.status.code() {
            Some(1223) => Err("The Windows admin prompt (UAC) was canceled.".to_string()),
            _ => Err(String::from_utf8_lossy(&out.stderr).into_owned()),
        }
    }
}

#[cfg(target_os = "macos")]
fn install_one(path: &str) -> Result<(), String> {
    // Adding to the System keychain's trust settings requires admin
    // authorization. Rather than requiring the whole app to run as root
    // or shipping a separate SMJobBless-style privileged helper tool, we
    // use osascript's "with administrator privileges" -- it triggers
    // macOS's native auth dialog (Touch ID or password) scoped to this
    // one shell command.
    //
    // CAVEAT: this nests two layers of quoting (POSIX shell, then
    // AppleScript string literal). The escaping below has been reasoned
    // through but not run against a real path containing spaces or
    // special characters -- test that before relying on it.
    let shell_cmd = format!(
        "security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain {}",
        shell_single_quote(path)
    );
    let osa_script = format!(
        "do shell script \"{}\" with administrator privileges",
        applescript_escape(&shell_cmd)
    );

    let out = Command::new("osascript")
        .args(["-e", &osa_script])
        .output()
        .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("User canceled") {
            Err("The macOS admin prompt was canceled.".to_string())
        } else {
            Err(stderr.into_owned())
        }
    }
}

/// Wraps a path in POSIX single quotes, safe for direct use in a shell
/// command string (handles embedded single quotes, spaces, etc).
#[cfg(target_os = "macos")]
fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Escapes a string for embedding inside an AppleScript double-quoted
/// string literal (used to wrap our already shell-quoted command for
/// `do shell script "..."`).
#[cfg(target_os = "macos")]
fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn install_one(_path: &str) -> Result<(), String> {
    Err("Unsupported platform".to_string())
}
