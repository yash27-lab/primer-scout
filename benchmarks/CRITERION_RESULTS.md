# Criterion Microbenchmark Results

Generated at: 2026-02-18 00:23:00Z

Command:

```bash
cargo bench --bench engine
```

Workload:
- Synthetic in-memory sequence: 1,000,000 bases
- Primer lengths: 20
- Primer panel sizes: 32 and 128
- Reverse-complement enabled

Results (from criterion output):

| Benchmark | Time (95% CI) | Throughput (95% CI) |
|---|---:|---:|
| `scan_sequence/primers_32/k0` | 39.636 ms - 40.895 ms | 24.061 - 23.320 MiB/s |
| `scan_sequence/primers_32/k1` | 63.663 ms - 65.281 ms | 14.980 - 14.609 MiB/s |
| `scan_sequence/primers_128/k0` | 140.01 ms - 145.11 ms | 6.8116 - 6.5720 MiB/s |
| `scan_sequence/primers_128/k1` | 239.03 ms - 247.62 ms | 3.9897 - 3.8513 MiB/s |

Notes:
- Numbers are from the current local machine and Rust toolchain.
- Re-run after code changes; do not treat as universal hardware-independent values.
