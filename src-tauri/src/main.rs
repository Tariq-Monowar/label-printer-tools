// FeetF1rst Print: a tray app that prints shipping labels from the admin panel
// straight to the label printer. The admin panel talks to it on 127.0.0.1:17771.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod printer;
mod server;

use serde::Serialize;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

pub const PORT: u16 = 17771;
const MAX_JOBS: usize = 15;
const TEST_LABEL: &[u8] = include_bytes!("../assets/test-label.pdf");

#[derive(Clone, Serialize)]
pub struct Job {
    title: String,
    at: u64,
    error: Option<String>,
}

#[derive(Default)]
pub struct AppState {
    pub printer: Mutex<String>,
    jobs: Mutex<Vec<Job>>,
    pub server_error: Mutex<Option<String>>,
}

#[derive(Serialize)]
struct Overview {
    printer: String,
    jobs: Vec<Job>,
    server_error: Option<String>,
    port: u16,
    version: String,
}

/// Prints a PDF with the chosen printer and adds the result to the job list.
pub fn print_job(app: &AppHandle, title: &str, pdf: &[u8]) -> Result<(), String> {
    let state = app.state::<AppState>();
    let name = state.printer.lock().unwrap().clone();
    let result = printer::print_pdf(app, pdf, &name);

    let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let mut jobs = state.jobs.lock().unwrap();
    jobs.insert(0, Job { title: title.to_string(), at, error: result.clone().err() });
    jobs.truncate(MAX_JOBS);
    let _ = app.emit("jobs", jobs.clone());
    result
}

fn printer_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join("printer.txt"))
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn overview(app: AppHandle) -> Overview {
    let state = app.state::<AppState>();
    let printer = state.printer.lock().unwrap().clone();
    let jobs = state.jobs.lock().unwrap().clone();
    let server_error = state.server_error.lock().unwrap().clone();
    Overview { printer, jobs, server_error, port: PORT, version: app.package_info().version.to_string() }
}

#[tauri::command]
async fn printers() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(printer::list).await.unwrap_or_default()
}

#[tauri::command]
fn set_printer(app: AppHandle, name: String) -> Result<(), String> {
    *app.state::<AppState>().printer.lock().unwrap() = name.clone();
    let file = printer_file(&app).ok_or("Kein Konfigurationsordner")?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, name).map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_print(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || print_job(&app, "Testdruck", TEST_LABEL))
        .await
        .map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        // a second start only brings the running app to the front
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_window(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![overview, printers, set_printer, test_print])
        .setup(|app| {
            let handle = app.handle().clone();

            if let Some(saved) = printer_file(&handle).and_then(|f| std::fs::read_to_string(f).ok()) {
                *handle.state::<AppState>().printer.lock().unwrap() = saved.trim().to_string();
            }
            #[cfg(not(debug_assertions))] // start with Windows (only the installed app)
            {
                use tauri_plugin_autostart::ManagerExt;
                let _ = handle.autolaunch().enable();
            }
            server::start(handle.clone());

            let open = MenuItem::with_id(app, "open", "Öffnen", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().expect("app icon"))
                .tooltip("FeetF1rst Print")
                .menu(&Menu::with_items(app, &[&open, &quit])?)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // started by Windows at login: stay in the tray
            if !std::env::args().any(|a| a == "--hidden") {
                show_window(&handle);
            }
            Ok(())
        })
        // closing the window keeps the app running in the tray
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("FeetF1rst Print failed to start");
}
