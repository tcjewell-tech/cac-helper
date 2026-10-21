use libloading::Library;
use serde::Serialize;
use std::os::raw::{c_ulong, c_void};
use std::path::PathBuf;

/// A known install location for a piece of CAC middleware, checked by name
/// so the diagnostics UI can tell the user which product it found.
#[derive(Serialize, Clone, Debug)]
pub struct MiddlewareCandidate {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Clone, Debug)]
pub enum MiddlewareOutcome {
    /// No known middleware found at any expected install location.
    NotFound,
    /// A file exists at a known location but failed the PKCS#11 sanity
    /// check (corrupt install, wrong CPU architecture, partial uninstall).
    FoundButInvalid,
    /// Found and passed a basic PKCS#11 C_GetFunctionList call.
    Valid,
}

#[derive(Serialize, Clone, Debug)]
pub struct MiddlewareResult {
    pub outcome: MiddlewareOutcome,
    pub found: Vec<MiddlewareCandidate>,
    /// Path of the first candidate that passed validation, if any. This is
    /// the path module 4 (Firefox NSS registration) will register.
    pub valid_path: Option<String>,
}

fn candidate(name: &str, path: &str) -> MiddlewareCandidate {
    MiddlewareCandidate {
        name: name.to_string(),
        path: path.to_string(),
    }
}

/// Known install locations for DoD-approved / common CAC middleware.
///
/// NOTE: these paths are best-effort defaults for the most common products
/// (OpenSC, HID/ActivClient, Thales/SafeNet). Verify against the current
/// DoD-approved middleware list before shipping -- vendors periodically
/// change default install paths between versions.
fn known_candidates() -> Vec<MiddlewareCandidate> {
    #[cfg(target_os = "windows")]
    {
        vec![
            candidate(
                "OpenSC",
                r"C:\Program Files\OpenSC Project\OpenSC\pkcs11\opensc-pkcs11.dll",
            ),
            candidate(
                "OpenSC (32-bit)",
                r"C:\Program Files (x86)\OpenSC Project\OpenSC\pkcs11\opensc-pkcs11.dll",
            ),
            candidate(
                "HID ActivClient",
                r"C:\Program Files\HID Global\ActivClient\acpkcs211.dll",
            ),
            candidate(
                "HID ActivClient (32-bit)",
                r"C:\Program Files (x86)\HID Global\ActivClient\acpkcs211.dll",
            ),
            candidate("Thales SafeNet", r"C:\Windows\System32\eTPKCS11.dll"),
        ]
    }
    #[cfg(target_os = "macos")]
    {
        vec![
            candidate(
                "OpenSC (Homebrew, Apple Silicon)",
                "/opt/homebrew/lib/opensc-pkcs11.so",
            ),
            candidate(
                "OpenSC (Homebrew, Intel)",
                "/usr/local/lib/opensc-pkcs11.so",
            ),
            candidate("OpenSC (pkg installer)", "/Library/OpenSC/lib/opensc-pkcs11.so"),
            candidate(
                "Thursby PKard",
                "/Library/Frameworks/PKard.framework/PKard",
            ),
        ]
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        vec![]
    }
}

type CkRv = c_ulong;
type GetFunctionListFn = unsafe extern "system" fn(*mut *mut c_void) -> CkRv;

/// Load the candidate library and call C_GetFunctionList -- the one
/// function every PKCS#11 module must export. We don't need the full
/// function table here, just proof this is a real, correctly-built
/// PKCS#11 module for the current OS/architecture (CKR_OK == 0).
fn validate_pkcs11(path: &str) -> bool {
    let lib = match unsafe { Library::new(path) } {
        Ok(l) => l,
        Err(_) => return false,
    };

    let get_function_list: libloading::Symbol<GetFunctionListFn> =
        match unsafe { lib.get(b"C_GetFunctionList\0") } {
            Ok(f) => f,
            Err(_) => return false,
        };

    let mut function_list_ptr: *mut c_void = std::ptr::null_mut();
    let rv = unsafe { get_function_list(&mut function_list_ptr) };

    rv == 0 && !function_list_ptr.is_null()
}

pub fn detect_middleware() -> MiddlewareResult {
    let candidates = known_candidates();
    let mut found = Vec::new();
    let mut valid_path = None;

    for c in &candidates {
        if PathBuf::from(&c.path).exists() {
            found.push(c.clone());
            if valid_path.is_none() && validate_pkcs11(&c.path) {
                valid_path = Some(c.path.clone());
            }
        }
    }

    let outcome = if valid_path.is_some() {
        MiddlewareOutcome::Valid
    } else if !found.is_empty() {
        MiddlewareOutcome::FoundButInvalid
    } else {
        MiddlewareOutcome::NotFound
    };

    MiddlewareResult {
        outcome,
        found,
        valid_path,
    }
}
