#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RUNS="${RUNS:-5}"
THREADS="${THREADS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || sysctl -n hw.ncpu)}"
BASES="${BASES:-5000000}"
PRIMER_COUNT="${PRIMER_COUNT:-128}"
PRIMER_LEN="${PRIMER_LEN:-20}"
SEED="${SEED:-42}"
OUT_DIR="benchmarks/generated"
RAW_CSV="$OUT_DIR/timings.csv"
RESULTS_MD="benchmarks/RESULTS.md"

mkdir -p "$OUT_DIR"

echo "[1/4] Building release binaries..."
cargo build --release

echo "[2/4] Generating deterministic synthetic dataset..."
./target/release/gen_synthetic \
  --reference-out "$OUT_DIR/reference.fa" \
  --primers-out "$OUT_DIR/primers.tsv" \
  --bases "$BASES" \
  --primer-count "$PRIMER_COUNT" \
  --primer-len "$PRIMER_LEN" \
  --seed "$SEED"

echo "run,seconds" > "$RAW_CSV"
CMD=(./target/release/primer-scout --primers "$OUT_DIR/primers.tsv" --reference "$OUT_DIR/reference.fa" --max-mismatches 1 --count-only --threads "$THREADS")

echo "[3/4] Running benchmark ${RUNS}x..."
for i in $(seq 1 "$RUNS"); do
  sec="$(
    {
      /usr/bin/time -p "${CMD[@]}" > /dev/null
    } 2>&1 | awk '/^real / {print $2}'
  )"
  echo "${i},${sec}" >> "$RAW_CSV"
  echo "  run ${i}: ${sec}s"
done

stats="$(awk -F, '
  NR > 1 {
    x = $2 + 0
    n += 1
    sum += x
    sumsq += x * x
    if (n == 1 || x < min) min = x
    if (n == 1 || x > max) max = x
  }
  END {
    mean = sum / n
    var = (sumsq / n) - (mean * mean)
    if (var < 0) var = 0
    std = sqrt(var)
    printf "%.6f,%.6f,%.6f,%.6f,%d", mean, min, max, std, n
  }
' "$RAW_CSV")"

IFS=',' read -r mean min max std n <<< "$stats"

cpu="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | xargs || echo unknown)"
ram_raw="$(sysctl -n hw.memsize 2>/dev/null || awk '/MemTotal/ {print $2 " KiB"}' /proc/meminfo || echo unknown)"
if [[ "$ram_raw" =~ ^[0-9]+$ ]]; then
  ram="$(awk -v bytes="$ram_raw" 'BEGIN {printf "%.2f GiB", bytes / (1024*1024*1024)}')"
else
  ram="$ram_raw"
fi
os_name="$(uname -srmo)"
utc_now="$(date -u +"%Y-%m-%d %H:%M:%SZ")"
throughput_mbp="$(awk -v bases="$BASES" -v mean="$mean" 'BEGIN {printf "%.3f", (bases / mean) / 1000000.0}')"

echo "[4/4] Writing benchmark report to ${RESULTS_MD}..."
cat > "$RESULTS_MD" <<EOF
# Benchmark Results

Generated at: ${utc_now}

## Environment

- OS: ${os_name}
- CPU: ${cpu}
- RAM: ${ram}
- Threads used: ${THREADS}
- Rust: $(rustc --version)
- Command: \`${CMD[*]}\`

## Dataset

- Bases: ${BASES}
- Primer count: ${PRIMER_COUNT}
- Primer length: ${PRIMER_LEN}
- Seed: ${SEED}

## Timing Summary (seconds)

- Runs: ${n}
- Mean: ${mean}
- Min: ${min}
- Max: ${max}
- Std dev: ${std}
- Throughput: ${throughput_mbp} million-bases/s

Raw timings CSV: \`${RAW_CSV}\`
EOF

echo "Benchmark complete."
