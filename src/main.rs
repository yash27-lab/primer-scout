use anyhow::{Context, Result};
use clap::Parser;
use primer_scout::{PrimerSummary, ScanOptions, load_primers, scan_references};
use serde::Serialize;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let primers = load_primers(&cli.primers)
        .with_context(|| format!("failed loading primers from '{}'", cli.primers.display()))?;

    let options = ScanOptions {
        max_mismatches: cli.max_mismatches,
        scan_reverse_complement: !cli.no_revcomp,
    };

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(cli.threads.max(1))
        .build()
        .context("failed to create rayon thread pool")?;

    let scan = pool.install(|| scan_references(&cli.references, &primers, &options))?;

    if cli.count_only {
        emit_count(scan.total_hits, cli.json)?;
    } else if cli.summary {
        emit_summary(&scan.summary, cli.json)?;
    } else {
        emit_hits(&scan.hits, cli.json)?;
    }

    Ok(())
}

#[derive(Debug, Parser)]
#[command(
    name = "primer-scout",
    version,
    about = "Fast Rust primer off-target scanner for FASTA references"
)]
struct Cli {
    /// Primer panel file (.tsv or .csv). Format: name<tab>sequence.
    #[arg(long, short = 'p')]
    primers: PathBuf,

    /// Reference FASTA file(s), plain text or .gz.
    #[arg(long = "reference", short = 'r', value_name = "FASTA", required = true)]
    references: Vec<PathBuf>,

    /// Allowed substitutions per hit.
    #[arg(long = "max-mismatches", short = 'k', default_value_t = 1)]
    max_mismatches: usize,

    /// Disable reverse-complement scanning.
    #[arg(long)]
    no_revcomp: bool,

    /// Emit one JSON object per line instead of TSV.
    #[arg(long)]
    json: bool,

    /// Output per-primer summary rows.
    #[arg(long)]
    summary: bool,

    /// Output only total number of hits.
    #[arg(long)]
    count_only: bool,

    /// Number of worker threads.
    #[arg(long, default_value_t = default_threads())]
    threads: usize,
}

fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
}

fn emit_hits(hits: &[primer_scout::Hit], as_json: bool) -> Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    for hit in hits {
        if as_json {
            writeln!(out, "{}", serde_json::to_string(hit)?)?;
        } else {
            writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                hit.file,
                hit.contig,
                hit.primer,
                hit.primer_len,
                hit.start,
                hit.end,
                hit.strand,
                hit.mismatches,
                hit.matched
            )?;
        }
    }
    out.flush()?;
    Ok(())
}

fn emit_summary(summary: &[PrimerSummary], as_json: bool) -> Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    for row in summary {
        if as_json {
            writeln!(out, "{}", serde_json::to_string(row)?)?;
        } else {
            writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                row.primer,
                row.primer_len,
                row.total_hits,
                row.perfect_hits,
                row.forward_hits,
                row.reverse_hits,
                row.contigs_with_hits
            )?;
        }
    }
    out.flush()?;
    Ok(())
}

fn emit_count(total: u64, as_json: bool) -> Result<()> {
    #[derive(Serialize)]
    struct CountRow {
        total_hits: u64,
    }

    let mut out = BufWriter::new(io::stdout().lock());
    if as_json {
        writeln!(
            out,
            "{}",
            serde_json::to_string(&CountRow { total_hits: total })?
        )?;
    } else {
        writeln!(out, "{total}")?;
    }
    out.flush()?;
    Ok(())
}
