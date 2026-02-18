# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

Added:
- `primer` launcher command for local interactive startup
- shared CLI core reused by both `primer` and `primer-scout` binaries
- startup update banner when a newer GitHub release is available
- `primer --splash` interactive full-screen console with persisted session history and restore

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
