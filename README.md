# Cadence ADHD

A Tauri v2 desktop application for Windows 11 providing ADHD-friendly daily routine management with gamified scoring, calendar integration, and a background reminder engine.

## Quick start (Windows 11)

```powershell
git clone https://github.com/SotSoG808/cadence-adhd.git
cd cadence-adhd
npm install
npm run dev          # cargo tauri dev
```

**Prerequisites**

| Tool | How to install |
|------|----------------|
| Rust stable ≥ 1.78 | `winget install Rustlang.Rustup` then `rustup default stable` |
| Node.js ≥ 20 LTS | `winget install OpenJS.NodeJS.LTS` |
| Tauri CLI v2 | `cargo install tauri-cli --version ^2` |
| MS C++ Build Tools | Visual Studio Installer → "Desktop development with C++" |
| WebView2 | Pre-installed on Windows 11 |

## Run tests

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

The automated tests cover: **due/overdue**, **scheduled-day**, **sequence**, **focus-mode** (Normal/Essentials/Quiet), **snooze**, **deferral**, **quiet flag**, **event-quiet hours**, **scoring/streak/level**, and all three **.ics datetime formats** (UTC Z, TZID Europe/London, floating) plus **EXDATE weekly recurrence**.

## Production build

```powershell
npm run build        # cargo tauri build
# MSI installer: src-tauri/target/release/bundle/msi/
```

## ZIP snapshot

```powershell
git archive --format=zip HEAD -o cadence-adhd.zip
```

## Architecture

```
src/                   Frontend (HTML / CSS / ES modules)
src-tauri/src/
  domain.rs            Pure-Rust task rules (eligible, due, overdue, snooze, defer)
  calendar.rs          .ics datetime parsing (UTC, TZID, floating) + EXDATE expander
  store/               Encrypted SQLite (AES-256-GCM, Argon2 key derivation)
  engine/              Background reminder loop (Tokio, survives window close)
  notification/        Windows toast dispatch + ntfy.sh mirror
  commands/            Tauri IPC commands exposed to frontend
  lib.rs               Tauri setup: tray, window-hide-on-close, command registry
```

## Features

| Screen | Key behaviour |
|--------|---------------|
| **Today** | Greeting, Now/Up-next task lanes, Done list, progress bar, snooze/defer buttons |
| **Routine** | Add fixed-time, flexible-block tasks; category; points |
| **Insights** | Daily pts/goal, streak, level, on-time % |
| **Calendars** | .ics import with RRULE/EXDATE/TZID/floating support |
| **Settings** | Focus mode, ntfy push, daily goal, data transparency notice |

- **Focus modes**: Normal / Essentials (meds, meals, care only) / Quiet
- **Persistence**: AES-256-GCM encrypted SQLite at `%APPDATA%\cadence-adhd\cadence.db`
- **Tray**: process stays alive when window is closed; click tray or Open Cadence to restore
- **SBOM**: generated on every CI run via `cargo-sbom` (see `docs/sbom-policy.md`)

## Acceptance testing

See [`docs/acceptance-tests.md`](docs/acceptance-tests.md).

## Pre-release checklist

See [`docs/sbom-policy.md`](docs/sbom-policy.md) for the full pre-release checklist (SBOM review, `cargo audit`, code-signing, accessibility audit).
