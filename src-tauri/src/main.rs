// Prevents an extra console window from appearing on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cert_store;
mod firefox_nss;
mod middleware;
mod reader;

use cert_store::{InstallReport, TrustCheckResult};
use firefox_nss::{NssCheckResult, RegisterReport};
use middleware::MiddlewareResult;
use reader::DetectionResult;

#[tauri::command]
fn check_reader() -> Result<DetectionResult, String> {
    reader::detect_readers().map_err(|e| e.to_string())
}

#[tauri::command]
fn check_middleware() -> MiddlewareResult {
    middleware::detect_middleware()
}

#[tauri::command]
fn check_trust() -> TrustCheckResult {
    cert_store::check_trust_status()
}

#[tauri::command]
fn open_dod_cert_page() -> Result<(), String> {
    open::that(cert_store::DOD_CERT_PAGE_URL).map_err(|e| e.to_string())
}

/// Frontend must confirm with the user before calling this -- see
/// cert_store::install_cert_files doc comment.
#[tauri::command]
fn install_certs(paths: Vec<String>) -> InstallReport {
    cert_store::install_cert_files(paths)
}

#[tauri::command]
fn check_nss(module_path: String) -> NssCheckResult {
    firefox_nss::check_nss_registration(&module_path)
}

/// Frontend must tell the user to close Firefox first -- see
/// firefox_nss::register_module doc comment.
#[tauri::command]
fn register_nss(profile_paths: Vec<String>, module_name: String, module_path: String) -> RegisterReport {
    firefox_nss::register_module(profile_paths, &module_name, &module_path)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            check_reader,
            check_middleware,
            check_trust,
            open_dod_cert_page,
            install_certs,
            check_nss,
            register_nss
        ])
        .run(tauri::generate_context!())
        .expect("error while running cac-helper");
}
