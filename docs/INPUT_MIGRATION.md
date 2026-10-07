# Input and output behavior notes

The scanner still emits the same nine hit fields and seven summary fields by
default, with 0-based half-open coordinates. Matching remains substitution-only,
IUPAC-aware, and reverse-complement-aware.

## Inputs now rejected explicitly

- Duplicate primer names: assign a unique identifier to each panel row.
- More than two panel columns: export just `name` and `sequence`, or a single
  sequence column. Embedded-delimiter quoted CSV is not implemented.
- Empty or control-containing primer/contig identifiers: provide a usable ID.
  FASTA descriptions after the first token are still ignored.
- FASTA files with no records: supply at least one named `>` record. A named
  empty contig is still accepted and produces zero hits.
- Non-ASCII primer bases: use supported ASCII IUPAC symbols. Whitespace within
  primer sequences is still ignored, lowercase is normalized, and U becomes T.
- Non-ASCII reference symbols or internal control characters: provide an ASCII
  reference. Unknown printable ASCII reference symbols retain the legacy N-like
  wildcard mask; this change does not make an ambiguity match a literal-base match.
- Public `Primer` fields edited after construction: rebuild the primer with
  `Primer::from_name_and_sequence` so its cached masks stay consistent.
- Both `--summary` and `--count-only`: choose one output mode.
- `--threads 0`: request a positive count. Large requests are still capped at
  four times the machine's available parallelism.

## Additive capabilities

UTF-8 BOMs at the start of primer/FASTA files are supported. Gzip magic bytes are
detected even without a `.gz` suffix; `.gz` inputs still use the multi-member
decoder. Primer errors report physical line numbers, including comment, blank,
and header lines.

Use `--header` for TSV hit/summary column names. It cannot be combined with
`--json` or `--count-only`. Closing a consumer pipe is normal termination;
input and non-pipe output failures still return errors.

Embedders can call `cli::try_run_from_args` to receive parsing errors, including
help/version requests, without terminating their process. Existing command-line
entry points retain Clap's normal argument handling.

## Parser allocation limits

Line limits are applied while reading, before a line String can grow without
bound. Primer reads also respect the remaining decompressed-file budget.
Limits include line endings and apply to headers/comments as well as data.
Invalid or non-positive environment values retain the documented defaults.

| Environment variable | Default |
| --- | --- |
| `PRIMER_SCOUT_MAX_PRIMER_FILE_BYTES` | 16 MiB |
| `PRIMER_SCOUT_MAX_PRIMER_LINE_BYTES` | 32 KiB |
| `PRIMER_SCOUT_MAX_FASTA_LINE_BYTES` | 8 MiB |
| `PRIMER_SCOUT_MAX_CONTIG_BASES` | 250,000,000 bases |

These are parser limits, not a total scan-memory cap. Hit vectors are retained
before output, including for summary/count-only modes. See the
[architecture](ARCHITECTURE.md) for accumulation and output contracts.
