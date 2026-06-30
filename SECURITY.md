# Security Policy

`primer-scout` is an offline command-line tool that processes user-supplied
FASTA references and primer panels. It is designed to be safe to run against
**untrusted input files**, but it is not a sandbox: treat it like any other
local CLI you run on your own machine.

## Supported Versions

Security fixes are applied to the latest `main` and the most recent tagged
release.

| Version | Supported |
| ------- | --------- |
| `main`  | ✅        |
| `0.1.x` | ✅        |

## Reporting a Vulnerability

Please **do not** open a public issue for a sensitive vulnerability.

Instead, use GitHub's private
[**Report a vulnerability**](https://github.com/yash27-lab/primer-scout/security/advisories/new)
flow, which keeps the report confidential until a fix is available.

When reporting, include:

- a description of the issue and its impact,
- the version or commit affected,
- and, where possible, a minimal input file or command that reproduces it.

You can expect an initial acknowledgement within a few days. Once a fix is
ready, it will be released and the reporter credited (unless anonymity is
requested).

## Hardening Already in Place

These protections ship by default:

- **Untrusted-input resource guards.** Per-line, per-file, and per-contig size
  limits bound memory and CPU use so a malformed or hostile file cannot trivially
  exhaust resources. Limits are tunable via the `PRIMER_SCOUT_MAX_*` environment
  variables (see the README).
- **Session-file confinement.** Console history is written under
  `$HOME/.primer-scout/` with restricted permissions (`0700` directory, `0600`
  file on Unix). A `PRIMER_SCOUT_SESSION_FILE` override is path-sanitized and
  cannot escape that directory, and symlinked targets are rejected.
- **Scanner resolution.** The interactive console invokes the scanner binary
  installed alongside it (via the current executable's directory) rather than
  resolving the bare name through `PATH`, avoiding binary-planting on platforms
  whose `PATH` search includes the current directory.
- **No telemetry.** The only outbound network request is an optional GitHub
  release check over HTTPS, which can be disabled with
  `PRIMER_SCOUT_NO_UPDATE_CHECK=1`.
- **Dependency auditing.** CI runs `cargo audit` against the advisory database
  on every push and pull request.

## Scope and Non-Goals

`primer-scout` is a research and engineering tool. It is **not** a medical
device and must not be used for clinical decision-making (see the README's
clinical disclaimer).
