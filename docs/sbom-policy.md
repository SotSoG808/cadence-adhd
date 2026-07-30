# SBOM Policy

Cadence generates a Software Bill of Materials (SBOM) on every CI run using `cargo-sbom`.

## Why

An SBOM provides a complete inventory of all Rust crate dependencies, their versions, and their licences. For a health-adjacent personal productivity tool this enables:

- Rapid identification of vulnerable dependencies (cross-reference against OSV / GHSA advisories)
- Licence compliance verification (all production crates are MIT/Apache-2.0)
- Supply-chain transparency if the app is later distributed

## Viewing the SBOM

The SBOM is uploaded as a CI artefact on every `main` push. Download it from the Actions tab in GitHub.

To generate locally:

```powershell
cargo install cargo-sbom
cargo sbom --manifest-path src-tauri/Cargo.toml > sbom.json
```

## Pre-release checklist

Before any public distribution:

- [ ] Review SBOM for dependency licence compatibility with intended release licence
- [ ] Run `cargo audit` to check for known CVEs
- [ ] Sign the MSI installer with a code-signing certificate
- [ ] Carry out accessibility audit (WCAG 2.1 AA) on all five screens
- [ ] Consider DPIA if personal health data scope broadens beyond single-user
