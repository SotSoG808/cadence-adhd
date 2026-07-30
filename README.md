# Cadence ADHD

Tauri v2 Windows 11 prototype implementation for ADHD lifestyle support.

## Clean clone

```powershell
git clone https://github.com/SotSoG808/cadence-adhd.git
cd cadence-adhd
npm install
npm run dev
```

Prerequisites: Rust stable, Node.js 20+, Microsoft C++ Build Tools/WebView2. Tests: `cargo test --manifest-path src-tauri/Cargo.toml`.

The tray process remains available after closing the window; choose **Quit** from its menu to terminate. See `docs/acceptance-tests.md` for manual verification and `tests/fixtures` for calendar fixtures.

## Security note

This prototype does not yet persist health data or send ntfy requests. Before a production deployment, add encrypted local persistence, explicit consent, an SBOM, signed releases, accessibility testing, Windows toast action integration tests, and a privacy/DPIA review.
