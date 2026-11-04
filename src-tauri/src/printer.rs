// Printer access. Windows prints silently through the bundled SumatraPDF,
// macOS / Linux through CUPS (`lp`). An empty printer name means the default printer.
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

pub fn list() -> Vec<String> {
    let output = if cfg!(windows) {
        hidden(Command::new("powershell").args([
            "-NoProfile",
            "-Command",
            "Get-Printer | Select-Object -ExpandProperty Name",
        ]))
        .output()
    } else {
        Command::new("lpstat").arg("-e").output()
    };
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default()
}

pub fn print_pdf(app: &AppHandle, pdf: &[u8], printer: &str) -> Result<(), String> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let file = std::env::temp_dir().join(format!("feetf1rst-label-{stamp}.pdf"));
    std::fs::write(&file, pdf).map_err(|e| e.to_string())?;
    let result = run(app, &file, printer);
    let _ = std::fs::remove_file(&file);
    result
}

#[cfg(windows)]
fn run(app: &AppHandle, file: &Path, printer: &str) -> Result<(), String> {
    use tauri::Manager;
    let sumatra = app.path().resource_dir().map_err(|e| e.to_string())?.join("bin").join("SumatraPDF.exe");
    let mut cmd = Command::new(sumatra);
    if printer.is_empty() {
        cmd.arg("-print-to-default");
    } else {
        cmd.args(["-print-to", printer]);
    }
    cmd.args(["-print-settings", "fit", "-silent"]).arg(file);
    check(hidden(&mut cmd).status())
}

#[cfg(not(windows))]
fn run(_app: &AppHandle, file: &Path, printer: &str) -> Result<(), String> {
    let mut cmd = Command::new("lp");
    if !printer.is_empty() {
        cmd.args(["-d", printer]);
    }
    cmd.args(["-o", "fit-to-page"]).arg(file);
    check(cmd.status())
}

fn check(status: std::io::Result<std::process::ExitStatus>) -> Result<(), String> {
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("Drucker meldet Fehler ({s})")),
        Err(e) => Err(format!("Druck nicht gestartet: {e}")),
    }
}

/// No console window flashing up on Windows.
fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}
