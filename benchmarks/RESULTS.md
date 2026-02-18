# Benchmark Results

Generated at: 2026-02-18 00:24:15Z

## Environment

- OS: Darwin 25.3.0 arm64
- CPU: Apple M2
- RAM: 8.00 GiB
- Threads used: 8
- Rust: rustc 1.93.1 (01f6ddf75 2026-02-11)
- Command: `./target/release/primer-scout --primers benchmarks/generated/primers.tsv --reference benchmarks/generated/reference.fa --max-mismatches 1 --count-only --threads 8`

## Dataset

- Bases: 5000000
- Primer count: 128
- Primer length: 20
- Seed: 42

## Timing Summary (seconds)

- Runs: 5
- Mean: 1.214000
- Min: 1.150000
- Max: 1.380000
- Std dev: 0.088227
- Throughput: 4.119 million-bases/s

Raw timings CSV: `benchmarks/generated/timings.csv`
