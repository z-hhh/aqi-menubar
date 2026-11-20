<h1 align="center">AQI Menubar</h1>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> · <b>English</b>
</p>

<p align="center">
  Air quality in your <b>macOS menu bar</b> or <b>Windows system tray</b> — one number, one click away
  from the station details. Data comes straight from <a href="https://aqicn.org">aqicn.org</a> with your
  own free API token: no account, no backend of ours, no telemetry.
</p>

<p align="center">
  <a href="https://github.com/z-hhh/aqi-menubar/releases/latest"><img src="https://img.shields.io/github/v/release/z-hhh/aqi-menubar?label=release" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/platforms-macOS%2011%2B%20%7C%20Windows%2010%2F11-blue" alt="Platforms">
  <img src="https://img.shields.io/badge/licence-MIT-green" alt="MIT licence">
  <a href="https://aqi.zhhh.fun"><img src="https://img.shields.io/badge/website-aqi.zhhh.fun-blue" alt="Landing page"></a>
</p>

<p align="center">
  <img src="docs/screenshots/panel-en.png" width="300" alt="The panel: an AQI badge with its level, the station name, the pollutant grid and the data sources">
  &nbsp;&nbsp;
  <img src="docs/screenshots/settings-en.png" width="300" alt="Settings: token, location, menu bar options with a live preview, refresh interval, language, updates">
</p>

| | |
|---|---|
| Platforms | macOS 11+ (Apple Silicon & Intel), Windows 10/11 |
| Stack | Tauri 2 · Rust · React + TypeScript · i18next |
| Languages | English · 简体中文 · 日本語 (auto-detected, switchable in Settings) |
| Licence | MIT |

## Install

Take the `.dmg` (macOS) or the installer (Windows) from
[Releases](https://github.com/z-hhh/aqi-menubar/releases/latest). Both builds update themselves in
place from the signed release feed.

The macOS build is ad-hoc signed, not notarised, so the first launch asks for confirmation:
right-click the app → **Open**, or allow it under **System Settings → Privacy & Security**.

A cask definition is kept in [`packaging/homebrew`](packaging/homebrew) for anyone who wants to
run a tap of their own. It is not published: without an Apple Developer ID (notarisation) Homebrew
installations hit the same Gatekeeper gate, so it is not worth a tap yet.

The landing page lives in [`landing/`](landing) and is published to GitHub Pages at
**<https://aqi.zhhh.fun>** by `.github/workflows/pages.yml` (custom domain via `zhhh.fun` on Cloudflare,
DNS-only CNAME to `z-hhh.github.io`).

## Screenshots

Shot from the running app at 2× — no mock-ups. The reading is live aqicn data for Shanghai.

| | |
|---|---|
| <img src="docs/screenshots/onboarding-en.png" width="250" alt="First run: a three-step token guide with Get a free token and I already have a token"> | <img src="docs/screenshots/panel-zh.png" width="250" alt="The panel in Chinese: the AQI badge with its level word, the station, pollutants and sources"> |
| **First run.** Three steps to a free token; “I already have a token” jumps straight to the field. | **The panel in 中文.** Same layout, localized level words and units. |

## What it does

- **Menu bar / tray**: configurable. Pick the AQI number, the PM2.5 number, or just a
  coloured dot; add the `AQI` label, the quality level (`51 · 良`, `51 · Moderate`) and the
  station name or not. The level word follows the interface language and is on by default.
  The text can take the level colour too, for a menu bar that reads at a glance without the
  dot; station names use the local name in the parentheses in Chinese and Japanese.
  Settings shows a live preview of the result, and the change lands in the menu bar as you
  save. (Windows cannot render text in the notification area, so there the dot is the icon and
  the numbers plus the level live in the tooltip.)
- **Panel** (click the tray item): AQI badge and category, station name, station reading time,
  PM2.5 / PM10 / O₃ / NO₂ / SO₂ / CO, main pollutant, temperature, humidity and wind, and the
  full list of upstream data sources as clickable links.
- **Location**: automatic (aqicn resolves it from your IP), a city or station name, or a
  specific station picked from search.
- **Refresh interval**: 5–60 minutes. aqicn stations publish hourly, so anything faster only
  burns your quota.
- **Updates**: checked 20 seconds after launch and then every six hours, installed in place
  from the signed release feed, with a **Check for updates…** item in the tray menu.
- Data stays on your machine. If a refresh fails, the panel keeps showing the last reading and
  marks it as stale instead of going blank.

## Getting a token

The app opens on a three-step setup screen the first time it runs:

1. Open <https://aqicn.org/data-platform/token/>.
2. Enter your email and name — the token arrives immediately by email.
3. Paste it into the app.

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
pnpm tauri build    # .app / .dmg or .msi / .exe in src-tauri/target/release/bundle
```

The panel's frontend also runs in a plain browser against mock data, which is how the UI is
reviewed without launching the desktop app:

```bash
pnpm dev   # http://localhost:1420/?mock=empty | ?mock=data&aqi=178 | ?update=0.2.0&lang=zh-CN
```

Checks:

```bash
pnpm build                      # type-check + frontend bundle
cd src-tauri && cargo test      # API parsing, URL building, colour bands, tray text
```

Two developer environment variables help with scripted checks and the screenshots in
[`docs/screenshots`](docs/screenshots): `AQI_MENUBAR_SHOW=1` opens the panel on launch, and
`AQI_MENUBAR_DEV_VIEW=settings` opens Settings directly. With a token in the config, the window
captures at 2× on a Retina display — that is how these images were taken.

### Layout

```
src/                  React panel UI, i18n resources (src/locales/*.json), mock backend
src-tauri/src/
  aqicn.rs            API client + response parsing (+ unit tests)
  config.rs           Settings model (location, refresh, language, tray display) and storage
  tray.rs             Tray icon, menu, panel positioning, menu bar text composition (+ tests)
  level.rs            AQI bands and their localised level words (tray, dot colour, panel)
  icons.rs            EPA colour bands and the generated tray dot
  commands.rs         IPC surface: state, config, search, tray preview, updater
  lib.rs              App setup, the polling loop, the update check
docs/screenshots/     README screenshots (2× PNG, also used by the landing page)
landing/              Static landing page (GitHub Pages)
packaging/homebrew/   Cask, tap bump script, official-cask checklist
```

Polling is a single async loop that wakes on a timer or when settings change, fetches, updates
the tray and pushes the new state to the UI over an event. The UI never polls by itself. The
update check runs in Rust on its own timer for the same reason.

The menu bar text is composed in exactly one place (`tray::texts`); `src/aqi.ts` `trayText()`
mirrors it for the Settings preview, and both have to be changed together. The bands themselves
live in `level.rs`; the frontend's `categoryOf()` in `src/aqi.ts` mirrors them for colours and
the panel.

### Releasing

Tag a commit (`git tag v0.2.0 && git push --tags`). `.github/workflows/release.yml` builds a
universal macOS bundle and a Windows installer, signs the updater artifacts and opens a draft
release with `latest.json`.

Repository secrets it needs:

| Secret | What for |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | signs updater artifacts (contents of `~/.tauri/aqi-menubar.key`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | its password (empty is fine) |

If the signing key is lost, existing installs cannot verify updates and everyone has to
download once by hand — keep it somewhere safe.

## Adding a language

1. Copy `src/locales/en.json` to `src/locales/<tag>.json` and translate the values.
2. Register it in `src/i18n.ts` (`resources` and `SUPPORTED`).
3. Add an `<option>` to the language selector in `src/Settings.tsx`.
4. Add the tag to `labels()` in `src-tauri/src/tray.rs` for the tray menu.

## Licence

MIT — see [LICENSE](LICENSE). “aqicn”, “World Air Quality Index” and the data belong to their
respective owners.
