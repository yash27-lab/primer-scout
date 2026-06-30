# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

Added:
- `primer` launcher command for local interactive startup
- shared CLI core reused by both `primer` and `primer-scout` binaries
- startup update banner when a newer GitHub release is available
- `primer --splash` interactive full-screen console with persisted session history and restore
- beginner console commands: `/basics`, `/examples`, `/upgrade`, `/version`, `/history`
- command suggestion overlay when typing `/` in console
- direct command passthrough in console (`primer-scout ...` and `--flags ...`)
- hardened console session storage (`0700`/`0600` perms on Unix, symlink rejection, sanitized session path override)
- input safety guardrails for untrusted data (line-size/file-size/contig-size limits with env overrides)
- thread cap hardening for user-provided `--threads`
- `SECURITY.md` security policy with private vulnerability-reporting channel
- `cargo audit` dependency-vulnerability scan as a CI job
- declared MSRV (`rust-version = "1.88"`) and an LTO-enabled release profile
- engine test coverage for IUPAC ambiguity matching and the `--no-revcomp` path
- prebuilt cross-platform release binaries (Linux, macOS Intel/Apple Silicon,
  Windows) published automatically on `v*` tags via a release workflow
- `docs/ARCHITECTURE.md` documenting the matching engine, parallelism, output
  contracts, and safety model
- black-box CLI integration test suite (`tests/cli.rs`) covering count/summary/JSON
  output, CSV and gzip inputs, `--no-revcomp`, and error/limit paths

Changed:
- console resolves the scanner binary next to the current executable instead of
  via `PATH`, avoiding binary-planting on platforms that search the working directory
- expanded README with a clear problem statement, tool-comparison table,
  open-source rationale, and prebuilt-binary install instructions
- extracted `SummaryAccumulator::merge` to remove the duplicated summary
  accumulation loops in the engine

Fixed:
- clippy `useless_conversion` errors that were failing CI under current stable
- corrected the placeholder `repository` URL in `Cargo.toml`

Security:
- updated dependencies to clear `cargo audit` advisories (`rustls-webpki`
  RUSTSEC-2026-0049/0098/0099/0104 and the `anyhow` RUSTSEC-2026-0190 unsoundness)

## [0.1.0] - 2026-02-18

Initial public release.

Added:
- Primer off-target scan engine for FASTA references
- Mismatch-tolerant matching and reverse-complement scanning
- CLI output modes: hits, summary, count-only
- TSV and NDJSON outputs
- Reproducible benchmark scripts and benchmark artifacts
- CI workflow, contribution guide, issue templates, and PR template

Release notes: `releases/v0.1.0.md`
