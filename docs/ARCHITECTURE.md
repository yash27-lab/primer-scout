# primer-scout architecture

This document describes how `primer-scout` is put together and how its matching
engine works. It is written for contributors and reviewers who want to change the
code with confidence, not for end users (see [`README.md`](../README.md) for usage).

Everything below is grounded in the source under `src/`. File and function names
are given so you can jump straight to the relevant code.

## Overview & crate layout

`primer-scout` is a single Cargo package (`Cargo.toml`) that produces one library
and several targets:

- **Library crate** (`src/lib.rs`) — the scanning engine. It owns the `Primer`,
  `ScanOptions`, `Hit`, `PrimerSummary`, and `ScanResult` types and the public
  entry points `load_primers`, `scan_references`, and `scan_sequence`. The CLI,
  console, update check, and splash animation are submodules
  (`pub mod cli; pub mod console; pub mod splash; pub mod update;`).
- **`primer-scout` binary** (`src/main.rs`) — the direct, non-interactive scanner.
  Its `main` is a one-liner that calls `cli::run()`. This is the package's
  `default-run`, so `cargo run` builds it.
- **`primer` binary** (`src/bin/primer.rs`) — the interactive console launcher.
  With no arguments (or a lone `--splash`) **and** a TTY on stdout, it runs the
  update check and hands off to `console::run`. Otherwise it forwards its argv to
  `cli::run_from_args`, so `primer --primers ... --reference ...` behaves like the
  direct scanner.
- **`gen-synthetic` binary** (`src/bin/gen_synthetic.rs`) — a deterministic
  generator that writes a synthetic FASTA reference plus a primer panel for
  benchmarking. It is reproducible from a seed (a small `XorShift64` PRNG) and is
  not part of the scanning path.
- **`engine` bench** (`benches/engine.rs`, declared `[[bench]] harness = false`) —
  a Criterion microbenchmark that drives `scan_sequence` over a 1M-base synthetic
  contig across panel sizes `{32, 128}` and mismatch budgets `{0, 1}`.

The dependency surface is intentionally small: `clap` (argv), `serde`/`serde_json`
(output + history), `rayon` (parallelism), `flate2` (gzip), `crossterm` (console
TUI), `ureq` + `semver` (update check), and `anyhow` for error context.

## The matching engine

### IUPAC 4-bit bitmask matching

Every base is represented as a 4-bit mask over the alphabet
`{A = 0b0001, C = 0b0010, G = 0b0100, T = 0b1000}`. An IUPAC ambiguity code is the
bitwise OR of the bases it stands for, so a single bit test covers ambiguity. The
mapping lives in `iupac_mask` in `src/lib.rs`; for example:

```
A = 0b0001   C = 0b0010   G = 0b0100   T = 0b1000
R = A|G = 0b0101          Y = C|T = 0b1010
S = C|G = 0b0110          W = A|T = 0b1001
K = G|T = 0b1100          M = A|C = 0b0011
B = C|G|T = 0b1110        D = A|G|T = 0b1101
H = A|C|T = 0b1011        V = A|C|G = 0b0111
N = A|C|G|T = 0b1111
```

Before masking, bases are normalised by `normalize_base`: `U`/`u` is folded to `T`
(so RNA primers work) and everything else is upper-cased. Primer sequences are
validated through `normalize_query` / `to_masks`, which reject any character that
is not a known IUPAC code. Reference bases are different: `mask_or_unknown` maps an
unknown reference byte to `N` (`0b1111`), i.e. it matches anything, rather than
failing the scan.

A position **matches** when the query and reference masks share at least one set
bit:

```
fn position_matches(query_mask: u8, seq_mask: u8) -> bool {
    (query_mask & seq_mask) != 0
}
```

This is the core test in `scan_orientation` (`src/lib.rs`); a zero AND means a
mismatch at that offset.

### Mismatch counting

Matching is a plain sliding window. For a window starting at `start`,
`scan_orientation` walks each offset, applies the bitmask test, and increments a
`mismatches` counter on each failure. The inner loop **early-terminates** the
moment `mismatches > max_mismatches`, so deeply non-matching windows are abandoned
quickly:

```
for (offset, &query_mask) in query_masks.iter().enumerate() {
    if (query_mask & sequence_masks[start + offset]) == 0 {
        mismatches += 1;
        if mismatches > max_mismatches { break; }
    }
}
```

A window with `mismatches <= max_mismatches` is recorded as a `Hit` (with
`mismatches == 0` additionally counted as a "perfect" hit). The reference is
pre-normalised once per contig into a `sequence_bytes` vector and a parallel
`sequence_masks` vector (`scan_contig`), so the hot loop only does mask ANDs and
counter bumps — no per-base re-parsing. Matching is ungapped: only substitutions
are counted, never insertions or deletions.

### Strand handling

By default each primer is scanned on both strands. The forward masks
(`primer.masks`) and the reverse-complement masks (`primer.reverse_masks`) are both
computed **once**, at primer construction time in `Primer::from_name_and_sequence`,
from the sequence and its `reverse_complement`. The reverse strand is therefore
just a second pass of the same sliding window over the precomputed reverse masks;
the reference is never reversed.

Reverse-complement scanning is gated by `ScanOptions::scan_reverse_complement`
(disabled with `--no-revcomp`). A primer whose reverse complement equals itself is
flagged `is_palindromic` at build time, and palindromic primers are scanned **only
on the forward strand** (`scan_primer_in_contig`) so a single physical site is not
double-counted as both `+` and `-`. `complement_base` defines the IUPAC complement
table used to build the reverse-complement string.

### Parallelism

Within a single contig, primers are scanned in parallel: `scan_contig` calls
`primers.par_iter().enumerate()` (rayon) and runs `scan_primer_in_contig` for each
primer, then reassembles the per-primer results back into contig order by index.
Contigs and reference files are processed sequentially as they stream in; the
parallel fan-out is across the panel, not across the genome.

The direct CLI (`src/cli.rs`) builds a **bounded** rayon thread pool and runs the
whole scan inside `pool.install(...)`. The requested `--threads` value (default:
`std::thread::available_parallelism()`) is clamped to at least 1 and at most
`available_parallelism() * MAX_THREAD_MULTIPLIER` (`MAX_THREAD_MULTIPLIER = 4`), so
a user cannot accidentally spawn an unbounded number of OS threads.

## FASTA streaming

References are read line by line (`scan_reference_file` in `src/lib.rs`) through a
`BufRead` returned by `open_reader`. If the path ends in `.gz` (case-insensitive),
the reader is wrapped in `flate2`'s `MultiGzDecoder`; otherwise the file is read as
plain text. The decoder type is boxed (`Box<dyn BufRead + Send>`) so both paths
share one code path.

Parsing is classic FASTA accumulation: a line beginning with `>` opens a new
contig (flushing and scanning the previous one), and subsequent non-empty lines are
appended to the current contig buffer. Sequence bytes appearing before any header
are a hard error. Contig names come from `parse_contig_name`, which takes the first
whitespace-delimited token after `>` (falling back to `unknown_contig`). Each
contig is scanned as soon as its next header (or EOF) is reached, so the whole file
is never held in memory at once — only one contig's sequence plus its mask vector.

## Coordinate system & output contracts

Coordinates are **0-based, half-open** `[start, end)`, where `end = start +
primer_len`. The `matched` field is the literal reference substring under the
window (normalised bases), not the primer.

There are three mutually exclusive output modes, selected in `cli::execute` and
each available as TSV (default) or NDJSON (`--json`, one JSON object per line):

- **Hit-level** (default) — one row per match. TSV columns, in emission order from
  `emit_hits`:

  ```
  file  contig  primer  primer_len  start  end  strand  mismatches  matched
  ```

- **Summary** (`--summary`) — one row per primer. TSV columns, in emission order
  from `emit_summary`:

  ```
  primer  primer_len  total_hits  perfect_hits  forward_hits  reverse_hits  contigs_with_hits
  ```

- **Count-only** (`--count-only`) — a single total. Plain mode prints the bare
  integer; JSON mode prints `{"total_hits": N}` (`emit_count`).

Ordering is deterministic. Hits are sorted by
`(file, contig, primer, start, strand, mismatches)` in `scan_references`, and
summary rows are sorted by primer name. The JSON field names are the `serde`
field names on `Hit` / `PrimerSummary` and match the TSV column order.

A note for contributors changing defaults: `ScanOptions::default()` uses
`max_mismatches = 0`, but the CLI flag `--max-mismatches` defaults to `1`. The two
are independent; the engine default is only used by callers that construct
`ScanOptions` directly.

## Input safety / resource guards

The engine is built to ingest untrusted files without being trivially driven into
unbounded memory or CPU use. Four limits are read from the environment via
`read_limit_from_env` (each falls back to a compiled-in default; a value must parse
as a positive `usize` to override):

| Env var | Default | Bounds |
| --- | --- | --- |
| `PRIMER_SCOUT_MAX_PRIMER_FILE_BYTES` | 16 MiB | Total bytes read from a primer file (`load_primers`). |
| `PRIMER_SCOUT_MAX_PRIMER_LINE_BYTES` | 32 KiB | Length of a single primer-file line. |
| `PRIMER_SCOUT_MAX_FASTA_LINE_BYTES` | 8 MiB | Length of a single FASTA line. |
| `PRIMER_SCOUT_MAX_CONTIG_BASES` | 250,000,000 | Accumulated bases per contig (and per in-memory sequence in `scan_sequence`). |

Exceeding any limit aborts with an error that names the override variable. Contig
growth is checked incrementally with a saturating add before each append, so the
buffer cannot overshoot the cap by more than one line.

The interactive console (`src/console.rs`) adds a session-file confinement model:

- History lives under `$HOME/.primer-scout/` (`console_history.ndjson`). On Unix the
  directory is forced to `0700` and the file is opened/`set_permissions`-ed to
  `0600` (`secure_directory_permissions`, `secure_file_permissions`,
  `open_history_file`).
- `PRIMER_SCOUT_SESSION_FILE` can relocate the history file, but
  `sanitize_history_override` confines it to that base directory: relative paths
  with `..`, root, or a drive prefix are rejected, and an absolute override must
  start with the base dir. Anything else falls back to the default path.
- Before reading or writing history, `reject_symlink` calls `symlink_metadata` and
  refuses to operate on a symlink, so the session file cannot be aimed at another
  file through a planted link.
- When the console runs a scan, `scanner_command` prefers the `primer-scout` binary
  sitting **next to the current executable** (`current_exe().parent()`), only
  falling back to bare-name `PATH` resolution if that sibling is missing. This
  avoids picking up an attacker-planted `primer-scout` on `PATH` (notably the CWD on
  Windows).

The only outbound network call is the optional GitHub release check in
`src/update.rs`: it queries the `releases/latest` API with short connect/read/write
timeouts and compares the tag (via `semver`) against the running version. It is
skipped entirely when `PRIMER_SCOUT_NO_UPDATE_CHECK` is set, and a banner is only
shown when a strictly newer version exists.

## Extending the engine

Pointers for common changes:

- **New ambiguity / alphabet handling** — edit `iupac_mask` (forward mapping),
  `complement_base` (reverse-complement table), and `normalize_base` (input
  folding) in `src/lib.rs`. Keep all three consistent.
- **Different matching semantics** (e.g. weighting, scoring) — the inner test lives
  in `scan_orientation`; the per-primer/per-strand orchestration is in
  `scan_primer_in_contig`. Note the palindrome guard there if you change strand
  behaviour.
- **New output fields or modes** — extend `Hit` / `PrimerSummary` in `src/lib.rs`
  (they derive `Serialize`, so JSON follows automatically) and update the matching
  `emit_*` writer and column list in `src/cli.rs`. Update the sort key in
  `scan_references` if ordering should change.
- **New CLI flags** — add to the `Cli` struct in `src/cli.rs` and thread the value
  into `ScanOptions` or the thread-pool setup in `execute`.
- **New console commands** — handle them in `handle_message` and register the name
  in the `CONSOLE_COMMANDS` table (`src/console.rs`) so suggestions pick it up.
- **Benchmarks** — `benches/engine.rs` (Criterion micro) drives `scan_sequence`
  directly; `src/bin/gen_synthetic.rs` produces deterministic macro-benchmark
  inputs.
