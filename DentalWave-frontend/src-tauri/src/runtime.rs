use crate::{database::AppState, error::AppError, storage};
use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfo {
    pub app_version: &'static str,
    pub platform: &'static str,
    pub architecture: &'static str,
    pub network_required: bool,
    pub startup_error: Option<AppError>,
}

#[tauri::command]
pub fn application_info(info: tauri::State<'_, ApplicationInfo>) -> &ApplicationInfo {
    info.inner()
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let result = (|| {
        let data = app.path().app_data_dir().map_err(|_| storage::failure("APPLICATION_DATA_DIRECTORY_UNAVAILABLE"))?;
        let logs = app.path().app_log_dir().map_err(|_| storage::failure("APPLICATION_DATA_DIRECTORY_UNAVAILABLE"))?;
        let default = if cfg!(debug_assertions) { storage::location::Mode::Local } else { storage::location::Mode::Portable };
        #[allow(unused_mut)]
        let mut locations = storage::location::DatabaseLocationService::new(data, logs, default)?;
        let executable = std::env::current_exe().map_err(|_| storage::failure("INVALID_PORTABLE_PACKAGE"))?;
        locations.portable_package_directory = storage::location::package_directory(&executable)?;
        if locations.portable_package_directory.is_some() {
            locations.settings.mode = storage::location::Mode::Portable;
        }
        #[cfg(debug_assertions)]
        if let Some(path) = std::env::var_os("DENTAL_SCHEDULER_MIGRATION_COPY") {
            let path = std::path::PathBuf::from(path);
            crate::database::Database::migrated_copy(path.clone())?;
            locations.local_override = Some(path);
            locations.settings.mode = storage::location::Mode::Local;
        }
        let mode = locations.settings.mode.clone();
        let mut manager = storage::StorageManager::new(locations);
        manager.event("APPLICATION_STARTED");
        // Storage errors render in Data; never substitute another database.
        let _ = manager.open(mode, None);
        app.manage(AppState { storage: std::sync::Mutex::new(manager) });
        Ok::<_, AppError>(())
    })();
    if let Err(error) = &result {
        if let Ok(logs) = app.path().app_log_dir() { storage::write_log(&logs, &error.code); }
    }
    app.manage(ApplicationInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        network_required: false,
        startup_error: result.err(),
    });
    let builder = tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
        .devtools(cfg!(debug_assertions))
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_navigation(|url| navigation_allowed(url, cfg!(debug_assertions)));
    // WebView2 profile/cache is local; never beside the EXE or on the data drive.
    #[cfg(target_os = "windows")]
    let builder = builder.data_directory(app.path().app_local_data_dir()?.join("WebView2"));
    builder.build()?;
    Ok(())
}

fn navigation_allowed(url: &tauri::Url, development: bool) -> bool {
    let local_app = url.port().is_none() && match (url.scheme(), url.host_str()) {
        ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost")) => true,
        _ => false,
    };
    local_app || (development && url.scheme() == "http" && url.host_str() == Some("localhost") && url.port() == Some(5173))
}

// If WebView2/window creation fails, HTML cannot display an error. Use the OS
// message box without shell execution; never display raw framework errors.
pub fn show_startup_failure() {
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "user32")]
        extern "system" { fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, caption: *const u16, flags: u32) -> i32; }
        let message: Vec<u16> = "Dental Scheduler Mini could not create its application window. Ask your support team to verify Microsoft Edge WebView2 and application-data folder access. No runtime was downloaded. Error code: APPLICATION_STARTUP_FAILED".encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = "Dental Scheduler Mini".encode_utf16().chain(Some(0)).collect();
        MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), 0x10);
    }
    eprintln!("APPLICATION_STARTUP_FAILED");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_navigation_accepts_only_embedded_origins() {
        for url in ["tauri://localhost/manager/schedule", "http://tauri.localhost/", "https://tauri.localhost/"] {
            assert!(navigation_allowed(&url.parse().unwrap(), false));
        }
        for url in ["https://example.com", "http://localhost:8080", "http://localhost:5173", "file:///tmp/schedule", "http://tauri.localhost:8080", "https://tauri.localhost.evil.test"] {
            assert!(!navigation_allowed(&url.parse().unwrap(), false));
        }
        assert!(navigation_allowed(&"http://localhost:5173/".parse().unwrap(), true));
    }
}
