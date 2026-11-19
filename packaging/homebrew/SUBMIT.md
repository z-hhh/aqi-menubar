# Getting into the official Homebrew Cask

A personal tap (`z-hhh/homebrew-aqi-menubar`) works today. The official
`Homebrew/homebrew-cask` repository is stricter; this is the checklist.

| Requirement | Status |
|---|---|
| Public GitHub repo with a stable release URL | ✅ after the first `v*` tag |
| A `.dmg` that contains a plain `.app` | ✅ Tauri produces `AQI Menubar_<version>_universal.dmg` |
| **Signed with a Developer ID Application certificate** | ❌ needs an Apple Developer account (¥688/year) |
| **Notarized and stapled** (`xcrun stapler validate`) | ❌ needs the certificate above |
| `brew audit --cask --new aqi-menubar` clean | after the two items above |
| `desc` under 80 chars, no "app" filler, no version in the name | ✅ in `Casks/aqi-menubar.rb` |
| `livecheck` that resolves to the newest tag | ✅ `:github_latest` |

Without notarization, `brew install --cask` installs an app that macOS refuses
to open with "Apple cannot check it for malicious software"; the tap works but
users have to right-click → Open once. That is the reason to get the
certificate, not a technical detail you can paper over.

Once the certificate exists, add these to the build job and the rest is a
normal `brew audit` + PR:

```
APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY,
APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID
```

Tauri signs and notarizes automatically when those are present.

Windows: there is no Homebrew equivalent; the NSIS installer from the release
page is the supported path. Winget would be the equivalent submission
(`winget-pkgs`), and it has the same signing question.
