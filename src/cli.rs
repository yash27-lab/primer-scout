use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use std::ffi::OsString;
use std::io::{self, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::{PrimerSummary, ScanOptions, load_primers, scan_references};

const MAX_THREAD_MULTIPLIER: usize = 4;

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    execute(cli)
}

pub fn run_from_args<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    execute(cli)
}

/// Parses and executes scanner arguments without exiting the caller's process.
/// Clap help/version requests are returned as clap::Error inside anyhow::Error.
pub fn try_run_from_args<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    execute(cli)
}

fn execute(cli: Cli) -> Result<()> {
    let primers = load_primers(&cli.primers)
        .with_context(|| format!("failed loading primers from '{}'", cli.primers.display()))?;

    let options = ScanOptions {
        max_mismatches: cli.max_mismatches,
        scan_reverse_complement: !cli.no_revcomp,
    };

    let max_threads = available_threads()
        .saturating_mul(MAX_THREAD_MULTIPLIER)
        .max(1);
    let threads = cli.threads.max(1).min(max_threads);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .context("failed to create rayon thread pool")?;

    let scan = pool.install(|| scan_references(&cli.references, &primers, &options))?;

    if cli.count_only {
        emit_count(scan.total_hits, cli.json)?;
    } else if cli.summary {
        emit_summary(&scan.summary, cli.json, cli.header)?;
    } else {
        emit_hits(&scan.hits, cli.json, cli.header)?;
    }

    Ok(())
}

#[derive(Debug, Parser)]
#[command(
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

    /// Include column headings in hit or summary TSV output.
    #[arg(long, conflicts_with_all = ["json", "count_only"])]
    header: bool,

    /// Output per-primer summary rows.
    #[arg(long, conflicts_with = "count_only")]
    summary: bool,

    /// Output only total number of hits.
    #[arg(long)]
    count_only: bool,

    /// Number of worker threads.
    #[arg(long, default_value_t = default_threads(), value_parser = parse_threads)]
    threads: usize,
}

fn parse_threads(value: &str) -> std::result::Result<usize, String> {
    let threads = value.parse::<usize>().map_err(|_| "threads must be a positive integer".to_string())?;
    if threads == 0 { return Err("threads must be a positive integer".into()); }
    Ok(threads)
}

fn default_threads() -> usize {
    available_threads()
}

fn available_threads() -> usize {
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1)
}

fn emit_hits(hits: &[crate::Hit], as_json: bool, header: bool) -> Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    if header { writeln!(out, "file\tcontig\tprimer\tprimer_len\tstart\tend\tstrand\tmismatches\tmatched")?; }
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

fn emit_summary(summary: &[PrimerSummary], as_json: bool, header: bool) -> Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    if header { writeln!(out, "primer\tprimer_len\ttotal_hits\tperfect_hits\tforward_hits\treverse_hits\tcontigs_with_hits")?; }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_api_returns_parse_errors_and_help_without_exiting() {
        let error = try_run_from_args(["primer-scout", "--unknown"]).unwrap_err();
        assert!(error.downcast_ref::<clap::Error>().is_some());
        let help = try_run_from_args(["primer-scout", "--help"]).unwrap_err();
        assert_eq!(help.downcast_ref::<clap::Error>().unwrap().kind(), clap::error::ErrorKind::DisplayHelp);
    }
}
