// Local API for the admin panel (127.0.0.1 only):
//   GET  /status                       -> { ok, version, printer }
//   POST /print { url, title }         -> downloads the label PDF and prints it
// Only the FeetF1rst admin websites may call it, so no other site can print.
use crate::{print_job, AppState, PORT};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Request, Response, Server};

const ORIGINS: &[&str] = &["https://admin.feetf1rst.tech", "https://development.admin.feetf1rst.tech"];
const MAX_PDF: u64 = 10 * 1024 * 1024;

#[derive(Deserialize)]
struct PrintBody {
    url: String,
    title: Option<String>,
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let server = match Server::http(("127.0.0.1", PORT)) {
            Ok(server) => server,
            Err(e) => {
                *app.state::<AppState>().server_error.lock().unwrap() = Some(format!("Port {PORT} belegt: {e}"));
                return;
            }
        };
        for request in server.incoming_requests() {
            let app = app.clone();
            std::thread::spawn(move || handle(&app, request));
        }
    });
}

fn allowed(origin: &str) -> bool {
    ORIGINS.contains(&origin) || origin.starts_with("http://localhost:")
}

fn handle(app: &AppHandle, mut request: Request) {
    let origin = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Origin"))
        .map(|h| h.value.to_string());

    let (status, body) = match origin.as_deref() {
        Some(o) if !allowed(o) => (403, json!({ "error": "Origin nicht erlaubt" })),
        _ => match (request.method(), request.url()) {
            (Method::Options, _) => (204, Value::Null),
            (Method::Get, "/status") => (200, status(app)),
            (Method::Post, "/print") => match print(app, &mut request) {
                Ok(()) => (200, json!({ "ok": true })),
                Err(e) => (500, json!({ "error": e })),
            },
            _ => (404, json!({ "error": "Nicht gefunden" })),
        },
    };

    let mut response = Response::from_string(if body.is_null() { String::new() } else { body.to_string() })
        .with_status_code(status)
        .with_header(header("Content-Type", "application/json"));
    if let Some(o) = origin.filter(|o| allowed(o)) {
        response.add_header(header("Access-Control-Allow-Origin", &o));
        response.add_header(header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"));
        response.add_header(header("Access-Control-Allow-Headers", "Content-Type"));
        // Chrome asks before a website talks to this computer
        response.add_header(header("Access-Control-Allow-Private-Network", "true"));
    }
    let _ = request.respond(response);
}

fn status(app: &AppHandle) -> Value {
    let printer = app.state::<AppState>().printer.lock().unwrap().clone();
    json!({ "ok": true, "version": app.package_info().version.to_string(), "printer": printer })
}

fn print(app: &AppHandle, request: &mut Request) -> Result<(), String> {
    let mut raw = String::new();
    request.as_reader().take(64 * 1024).read_to_string(&mut raw).map_err(|e| e.to_string())?;
    let body: PrintBody = serde_json::from_str(&raw).map_err(|_| "Ungültige Anfrage".to_string())?;
    if !body.url.starts_with("https://") {
        return Err("Label-URL muss https sein".into());
    }

    let mut pdf = Vec::new();
    ureq::get(&body.url)
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|e| format!("Label nicht geladen: {e}"))?
        .into_reader()
        .take(MAX_PDF)
        .read_to_end(&mut pdf)
        .map_err(|e| e.to_string())?;
    if !pdf.starts_with(b"%PDF") {
        return Err("Die Datei ist kein PDF".into());
    }

    print_job(app, body.title.as_deref().unwrap_or("Label"), &pdf)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid header")
}
