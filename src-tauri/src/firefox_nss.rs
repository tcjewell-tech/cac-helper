use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Firefox keeps its own NSS certificate/module database per profile,
/// entirely separate from the OS trust store and OS-level PKCS#11
/// awareness. A middleware module that Windows/macOS already sees (module
/// 2) is NOT automatically visible to Firefox -- it has to be registered
/// into each profile's NSS module list individually.
#[derive(Serialize, Clone, Debug)]
pub struct FirefoxProfile {
    pub name: String,
    pub path: String,
    pub module_registered: bool,
}

#[derive(Serialize, Clone, Debug)]
pub enum NssOutcome {
    /// No Firefox install / profiles.ini found.
    NoProfilesFound,
    /// Firefox is installed, but the module isn't registered in any profile.
    ModuleNotRegisteredAnywhere,
    /// Registered in some profiles but not others (common if the user has
    /// multiple profiles, e.g. work vs. personal).
    PartiallyRegistered,
    /// Registered everywhere Firefox looks.
    RegisteredInAllProfiles,
}

#[derive(Serialize, Clone, Debug)]
pub struct NssCheckResult {
    pub outcome: NssOutcome,
    pub profiles: Vec<FirefoxProfile>,
}

/// Firefox's per-OS profile root. We read profiles.ini from here rather
/// than guessing folder names, since Firefox itself treats that file as
/// the source of truth.
fn firefox_root_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("Mozilla").join("Firefox"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("Firefox")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

/// Minimal profiles.ini parser -- just enough to pull Name/Path/IsRelative
/// out of each [Profile*] section. Not a general INI parser.
fn parse_profile_paths(root: &Path) -> Vec<(String, PathBuf)> {
    let content = match std::fs::read_to_string(root.join("profiles.ini")) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut profiles = Vec::new();
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut is_relative = true;

    let flush = |name: &mut Option<String>, path: &mut Option<String>, is_relative: bool, out: &mut Vec<(String, PathBuf)>| {
        if let (Some(n), Some(p)) = (name.take(), path.take()) {
            let full = if is_relative { root.join(&p) } else { PathBuf::from(&p) };
            out.push((n, full));
        }
    };

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            flush(&mut name, &mut path, is_relative, &mut profiles);
            is_relative = true;
        } else if let Some(v) = line.strip_prefix("Name=") {
            name = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("Path=") {
            path = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("IsRelative=") {
            is_relative = v.trim() == "1";
        }
    }
    flush(&mut name, &mut path, is_relative, &mut profiles);

    profiles
}

/// Checks whether `module_path` is already registered in a profile.
///
/// UNVERIFIED, flag before relying on this: with the modern SQLite NSS
/// database (sql: dbdir, the default for current Firefox), module
/// registration lives in a plain-text `pkcs11.txt` in the profile folder.
/// This does a simple substring check against that file. Confirm the
/// exact format against a real profile's pkcs11.txt before shipping --
/// if Firefox ever falls back to the legacy DBM `secmod.db` format this
/// check will silently report "not registered" even when it is.
fn module_registered_in_profile(profile_dir: &Path, module_path: &str) -> bool {
    match std::fs::read_to_string(profile_dir.join("pkcs11.txt")) {
        Ok(content) => content.contains(module_path),
        Err(_) => false,
    }
}

pub fn check_nss_registration(module_path: &str) -> NssCheckResult {
    let Some(root) = firefox_root_dir() else {
        return NssCheckResult {
            outcome: NssOutcome::NoProfilesFound,
            profiles: vec![],
        };
    };

    let raw_profiles = parse_profile_paths(&root);
    if raw_profiles.is_empty() {
        return NssCheckResult {
            outcome: NssOutcome::NoProfilesFound,
            profiles: vec![],
        };
    }

    let profiles: Vec<FirefoxProfile> = raw_profiles
        .into_iter()
        .map(|(name, path)| {
            let registered = module_registered_in_profile(&path, module_path);
            FirefoxProfile {
                name,
                path: path.to_string_lossy().into_owned(),
                module_registered: registered,
            }
        })
        .collect();

    let registered_count = profiles.iter().filter(|p| p.module_registered).count();
    let outcome = if registered_count == 0 {
        NssOutcome::ModuleNotRegisteredAnywhere
    } else if registered_count < profiles.len() {
        NssOutcome::PartiallyRegistered
    } else {
        NssOutcome::RegisteredInAllProfiles
    };

    NssCheckResult { outcome, profiles }
}

#[derive(Serialize, Clone, Debug)]
pub struct RegisterReport {
    pub succeeded: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Registers a PKCS#11 module into the given Firefox profiles via
/// `modutil`, the NSS tool Mozilla documents for this exact task
/// (`modutil -add <name> -libfile <path> -dbdir sql:<profile>`).
///
/// TWO UNSOLVED GAPS, same honesty pattern as modules 2/3 -- don't treat
/// this as production-ready without addressing them:
///   1. modutil ships with the separate NSS tools package, NOT with
///      Firefox itself. This app will need to bundle modutil (and its
///      NSS shared-library dependencies) or require a separate install.
///      Bundling third-party NSS binaries needs the same
///      verify-before-ship treatment as the middleware paths in module 2.
///   2. modutil needs the NSS database unlocked -- Firefox must be
///      closed first, or the call fails. The UI must tell the user to
///      close Firefox before triggering this.
pub fn register_module(
    profile_paths: Vec<String>,
    module_name: &str,
    module_path: &str,
) -> RegisterReport {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for profile in profile_paths {
        let dbdir = format!("sql:{}", profile);
        let out = Command::new("modutil")
            .args([
                "-force",
                "-dbdir",
                &dbdir,
                "-add",
                module_name,
                "-libfile",
                module_path,
            ])
            .output();

        match out {
            Ok(o) if o.status.success() => succeeded.push(profile),
            Ok(o) => failed.push((profile, String::from_utf8_lossy(&o.stderr).into_owned())),
            Err(e) => failed.push((profile, format!("modutil not found or failed to run: {e}"))),
        }
    }

    RegisterReport { succeeded, failed }
}
