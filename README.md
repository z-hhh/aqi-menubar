# AQI Menubar

Air quality in your **macOS menu bar** or **Windows system tray** — one number, one click
away from the station details. Data comes straight from [aqicn.org](https://aqicn.org) with
your own free API token: no account, no backend of ours, no telemetry.

| | |
|---|---|
| Platforms | macOS 11+ (Apple Silicon & Intel), Windows 10/11 |
| Stack | Tauri 2 · Rust · React + TypeScript · i18next |
| Languages | English · 简体中文 · 日本語 (auto-detected, switchable in Settings) |
| Licence | MIT |

<img src="assets/app-icon.png" width="96" alt="App icon">

## What it does

- **Menu bar / tray**: the current AQI as a number (macOS) or as an EPA-coloured dot with the
  number in the tooltip (Windows, which cannot render text in the notification area).
- **Panel** (click the tray item): AQI badge and category, station name, station reading time,
  PM2.5 / PM10 / O₃ / NO₂ / SO₂ / CO, main pollutant, temperature, humidity and wind, and the
  full list of upstream data sources as clickable links.
- **Location**: automatic (aqicn resolves it from your IP), a city or station name, or a
  specific station picked from search.
- **Refresh interval**: 5–60 minutes. aqicn stations publish hourly, so anything faster only
  burns your quota.
- Data stays on your machine. If a refresh fails, the panel keeps showing the last reading and
  marks it as stale instead of going blank.

## Getting a token

1. Open <https://aqicn.org/data-platform/token/>.
2. Enter your email and name — the token arrives immediately by email.
3. Paste it into **Settings** in the app.

The token is stored in plain text in the app's config file (permissions `0600`):

- macOS: `~/Library/Application Support/app.aqimenubar.desktop/config.json`
- Windows: `%APPDATA%\app.aqimenubar.desktop\config.json`

## Data source, attribution and terms

The readings are produced by the [World Air Quality Index project](https://waqi.info/) and its
contributing monitoring agencies, and served through the aqicn.org data platform API. Per the
platform's terms:

- This app is **free** and does not sell, resell or bundle the data.
- The **data sources are attributed in the panel**, using the `attributions` field returned by
  the API — including the monitoring agency that produced the reading.
- Readings are fetched live by each client with the user's own token and are **not cached or
  archived for redistribution**; the in-memory last reading exists only so the UI can show a
  stale value while a refresh is failing.
- The app is **not affiliated with or endorsed by** the World Air Quality Index project.

If you fork this and run it as a public or commercial service, read the aqicn.org API terms
yourself first — a hosted proxy that serves many users from one token is not covered by them.

## Development

Requirements: Node 20+, pnpm, Rust stable, and on macOS the Xcode command line tools
(on Windows, the MSVC build tools + WebView2).

```bash
pnpm install
pnpm tauri dev      # hot-reloading desktop app
pnpm tauri build    # produce .app / .dmg or .msi / .exe in src-tauri/target/release/bundle
```

Checks:

```bash
pnpm build                      # type-check + frontend bundle
cd src-tauri && cargo test      # API parsing, URL building, colour bands
```

### Layout

```
src/                  React panel UI, i18n resources (src/locales/*.json)
src-tauri/src/
  aqicn.rs            API client + response parsing (+ unit tests)
  config.rs           Settings model and persistence
  tray.rs             Tray icon, menu, panel positioning
  icons.rs            EPA colour bands and the generated tray dot
  commands.rs         IPC surface for the UI
  lib.rs              App setup and the polling loop
```

Polling is a single async loop that wakes on a timer or when settings change, fetches, updates
the tray and pushes the new state to the UI over an event. The UI never polls by itself.

## Adding a language

1. Copy `src/locales/en.json` to `src/locales/<tag>.json` and translate the values.
2. Register it in `src/i18n.ts` (`resources` and `SUPPORTED`).
3. Add an `<option>` to the language selector in `src/Settings.tsx`.
4. Add the tag to `labels()` in `src-tauri/src/tray.rs` for the tray menu.

## Licence

MIT — see [LICENSE](LICENSE). "aqicn", "World Air Quality Index" and the data belong to their
respective owners.
