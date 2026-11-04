# FeetF1rst Print

Small Windows tray app that prints shipping labels from the FeetF1rst admin panel
straight to the label printer (e.g. D450, 101.6 × 152.4 mm). No print dialog, no cutting.

## How it works

```
Admin panel (browser)  ──POST http://127.0.0.1:17771/print { url, title }──▶  FeetF1rst Print
                                                                               │ downloads the label PDF
                                                                               ▼
                                                                    SumatraPDF -silent ──▶ D450
```

- Runs in the tray, starts with Windows, ~8 MB installer.
- Listens only on `127.0.0.1:17771` (not reachable from other computers).
- Accepts requests only from `https://admin.feetf1rst.tech`, `https://development.admin.feetf1rst.tech` and `http://localhost:*`.
- Admin side: `admin/src/lib/labelPrinter.js` → `printLabel(url, title)`. If the app is not running, the label opens in a new tab.

| Endpoint | |
|---|---|
| `GET /status` | `{ ok, version, printer }` |
| `POST /print` | body `{ url, title }`, `url` must be an https PDF |

## Code

| File | |
|---|---|
| `src-tauri/src/main.rs` | app, tray, settings, job list |
| `src-tauri/src/server.rs` | local HTTP API |
| `src-tauri/src/printer.rs` | list printers, silent print (SumatraPDF on Windows, `lp` on macOS) |
| `ui/` | settings window (printer, test print, last prints) |

## Build

The Windows installer is built by GitHub Actions (`.github/workflows/build.yml`) on every push.
Tag `v1.0.1` (bump `version` in `src-tauri/tauri.conf.json` and `Cargo.toml` first) to publish a Release.

Local run (macOS / Windows with Rust installed):

```
npm ci
npm run dev
```

## Setup on the shipping PC

1. Install `FeetF1rst Print_x.y.z_x64-setup.exe`.
2. Choose the label printer, press **Testdruck**.
3. Printer driver paper size: 101.6 × 152.4 mm.
4. First Label click in Chrome: allow access to local devices once.
