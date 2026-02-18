# primer-scout

![primer-scout social preview](assets/social-preview-v4.png)

`primer-scout` is a high-speed Rust CLI for **primer off-target scanning** on FASTA references.

It is built for teams that need:
- fast preflight checks before expensive downstream workflows
- reproducible and script-friendly results
- simple deployment (single binary, no Python runtime required)

## Why This Exists

Many labs and bio teams face the same workflow gap:
- BLAST/aligners are powerful but heavy for quick panel screening loops
- grep-like tools are fast but usually lack primer-aware summary outputs
- ad-hoc scripts are hard to maintain and benchmark consistently

`primer-scout` targets that middle layer: quick, repeatable, production-friendly primer specificity checks.

## Is It Solving What People Want?

Yes, for these real needs:
- "Check panel specificity fast across one or many references."
- "Allow a controlled mismatch threshold and inspect all hit coordinates."
- "Integrate into CI/pipelines with stable machine-readable output."

No, for these advanced needs (out of scope today):
- thermodynamic scoring (Tm, dimer, hairpin)
- gapped alignment or indel-aware search
- full amplicon pair simulation and product-size modeling

Use `primer-scout` as a **high-throughput screening layer**, then move shortlisted candidates to deeper tools.

## Core Features

- Rust single-binary CLI
- FASTA input, including `.gz`
- Primer panel input as TSV/CSV
- IUPAC-aware matching (`A C G T/U R Y S W K M B D H V N`)
- Configurable mismatch threshold (`--max-mismatches`)
- Reverse-complement scanning enabled by default
- Parallel execution (`--threads`)
- Hit-level row output
- Per-primer summary output
- Count-only output
- TSV or NDJSON output

## Real Use Cases

1. Primer panel preflight before sequencing runs:
   Scan host genome + contamination references and rank primers by hit burden.
2. Off-target triage in assay development:
   Quickly identify primers with many near-matches at `k=1` or `k=2`.
3. CI regression checks:
   Re-run panel scans when primer files change and fail builds if hit counts jump.
4. Multi-reference screening:
   Test one panel against many assemblies without rewriting custom scripts.
5. Large-scale filtering:
   Use count-only mode for fast dashboards and trend tracking.

## Installation

```bash
cargo install --path .
```

## Input Format

Primer file (`.tsv` or `.csv`):

```text
name	sequence
primer_1	ATGCCGTAGCTA
primer_2	TTYACCGGTTAA
```

`name` is optional. If missing, names are auto-generated.

Reference input:
- one or more FASTA files with `--reference`
- plain or `.gz`

## Quick Start

Hit-level scan:

```bash
primer-scout \
  --primers data/demo_primers.tsv \
  --reference data/demo.fa \
  --max-mismatches 1
```

Per-primer summary:

```bash
primer-scout \
  --primers data/demo_primers.tsv \
  --reference data/demo.fa \
  --summary
```

Count-only:

```bash
primer-scout \
  --primers data/demo_primers.tsv \
  --reference data/demo.fa \
  --count-only
```

JSON output:

```bash
primer-scout \
  --primers data/demo_primers.tsv \
  --reference data/demo.fa \
  --summary \
  --json
```

Disable reverse-complement scanning:

```bash
primer-scout \
  --primers data/demo_primers.tsv \
  --reference data/demo.fa \
  --no-revcomp
```

## Output Contracts

Hit-level TSV columns:

```text
file  contig  primer  primer_len  start  end  strand  mismatches  matched
```

Summary TSV columns:

```text
primer  primer_len  total_hits  perfect_hits  forward_hits  reverse_hits  contigs_with_hits
```

Coordinate system: 0-based, half-open `[start, end)`.

## Performance And Benchmarking

This project does not claim performance without reproducible evidence.

Run the macro benchmark:

```bash
./scripts/run_benchmark.sh
```

Artifacts produced:
- `benchmarks/RESULTS.md`
- `benchmarks/generated/timings.csv`

Run microbenchmarks:

```bash
cargo bench --bench engine
```

Artifacts:
- `benchmarks/CRITERION_RESULTS.md`

Latest local macro run (2026-02-18, Apple M2, 8 threads):
- dataset: 5,000,000 bases, 128 primers, length 20, `k=1`
- mean runtime: 1.214 s
- throughput: 4.119 million-bases/s

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Project Ops

- Contributing guide: `CONTRIBUTING.md`
- Changelog: `CHANGELOG.md`
- Release notes: `releases/v0.1.0.md`
- GitHub launch metadata: `docs/github-launch.md`

## License

MIT
