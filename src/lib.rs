use anyhow::{Context, Result, bail};
use flate2::read::MultiGzDecoder;
use rayon::prelude::*;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub mod cli;
pub mod splash;

#[derive(Debug, Clone)]
pub struct Primer {
    pub name: String,
    pub sequence: String,
    pub reverse_complement: String,
    masks: Vec<u8>,
    reverse_masks: Vec<u8>,
    is_palindromic: bool,
}

impl Primer {
    pub fn len(&self) -> usize {
        self.sequence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sequence.is_empty()
    }

    pub fn from_name_and_sequence(name: impl Into<String>, sequence: &str) -> Result<Self> {
        let normalized = normalize_query(sequence)?;
        if normalized.is_empty() {
            bail!("primer sequence must not be empty");
        }

        let reverse_complement = reverse_complement(&normalized)?;
        let masks = to_masks(&normalized)?;
        let reverse_masks = to_masks(&reverse_complement)?;

        Ok(Self {
            name: name.into(),
            sequence: normalized.clone(),
            reverse_complement: reverse_complement.clone(),
            masks,
            reverse_masks,
            is_palindromic: normalized == reverse_complement,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_mismatches: usize,
    pub scan_reverse_complement: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_mismatches: 0,
            scan_reverse_complement: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub file: String,
    pub contig: String,
    pub primer: String,
    pub primer_len: usize,
    pub start: usize,
    pub end: usize,
    pub strand: char,
    pub mismatches: usize,
    pub matched: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrimerSummary {
    pub primer: String,
    pub primer_len: usize,
    pub total_hits: u64,
    pub perfect_hits: u64,
    pub forward_hits: u64,
    pub reverse_hits: u64,
    pub contigs_with_hits: u64,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub hits: Vec<Hit>,
    pub summary: Vec<PrimerSummary>,
    pub total_hits: u64,
}

pub fn load_primers(path: &Path) -> Result<Vec<Primer>> {
    let mut reader = open_reader(path)?;
    let mut line = String::new();
    let mut primers = Vec::new();
    let mut delimiter: Option<char> = None;
    let mut row_index = 0usize;

    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .with_context(|| format!("failed reading primer file '{}'", path.display()))?
            == 0
        {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let del = delimiter.unwrap_or_else(|| infer_delimiter(trimmed));
        delimiter = Some(del);
        let parts: Vec<&str> = trimmed.split(del).map(str::trim).collect();
        row_index += 1;

        let (name_raw, seq_raw) = if parts.len() >= 2 {
            (parts[0], parts[1])
        } else {
            ("", parts[0])
        };

        if row_index == 1 && is_header(name_raw, seq_raw) {
            continue;
        }

        let name = if name_raw.is_empty() {
            format!("primer_{:04}", primers.len() + 1)
        } else {
            name_raw.to_string()
        };
        let primer = Primer::from_name_and_sequence(name, seq_raw).with_context(|| {
            format!(
                "invalid primer sequence at row {} in '{}'",
                row_index,
                path.display()
            )
        })?;
        primers.push(primer);
    }

    if primers.is_empty() {
        bail!("no primers found in '{}'", path.display());
    }

    Ok(primers)
}

pub fn scan_references(
    references: &[PathBuf],
    primers: &[Primer],
    options: &ScanOptions,
) -> Result<ScanResult> {
    if references.is_empty() {
        bail!("no reference files supplied");
    }
    if primers.is_empty() {
        bail!("no primers supplied");
    }

    let mut merged_hits = Vec::new();
    let mut summary_acc = vec![SummaryAccumulator::default(); primers.len()];
    let mut total_hits = 0u64;

    for reference in references {
        let file_result = scan_reference_file(reference, primers, options)?;
        total_hits += file_result.total_hits;
        merged_hits.extend(file_result.hits);

        for (acc, delta) in summary_acc.iter_mut().zip(file_result.summary.into_iter()) {
            acc.total_hits += delta.total_hits;
            acc.perfect_hits += delta.perfect_hits;
            acc.forward_hits += delta.forward_hits;
            acc.reverse_hits += delta.reverse_hits;
            acc.contigs_with_hits += delta.contigs_with_hits;
        }
    }

    merged_hits.sort_by(|a, b| {
        (
            &a.file,
            &a.contig,
            &a.primer,
            a.start,
            a.strand,
            a.mismatches,
        )
            .cmp(&(
                &b.file,
                &b.contig,
                &b.primer,
                b.start,
                b.strand,
                b.mismatches,
            ))
    });

    let mut summary = primers
        .iter()
        .zip(summary_acc)
        .map(|(primer, acc)| PrimerSummary {
            primer: primer.name.clone(),
            primer_len: primer.len(),
            total_hits: acc.total_hits,
            perfect_hits: acc.perfect_hits,
            forward_hits: acc.forward_hits,
            reverse_hits: acc.reverse_hits,
            contigs_with_hits: acc.contigs_with_hits,
        })
        .collect::<Vec<_>>();

    summary.sort_by(|a, b| a.primer.cmp(&b.primer));

    Ok(ScanResult {
        hits: merged_hits,
        summary,
        total_hits,
    })
}

pub fn scan_sequence(
    sequence: &str,
    contig_name: &str,
    primers: &[Primer],
    options: &ScanOptions,
) -> Result<ScanResult> {
    if primers.is_empty() {
        bail!("no primers supplied");
    }

    let contig = scan_contig("in-memory", contig_name, sequence, primers, options)?;

    let mut summary = primers
        .iter()
        .zip(contig.summary)
        .map(|(primer, acc)| PrimerSummary {
            primer: primer.name.clone(),
            primer_len: primer.len(),
            total_hits: acc.total_hits,
            perfect_hits: acc.perfect_hits,
            forward_hits: acc.forward_hits,
            reverse_hits: acc.reverse_hits,
            contigs_with_hits: acc.contigs_with_hits,
        })
        .collect::<Vec<_>>();
    summary.sort_by(|a, b| a.primer.cmp(&b.primer));

    Ok(ScanResult {
        hits: contig.hits,
        summary,
        total_hits: contig.total_hits,
    })
}

fn scan_reference_file(
    reference: &Path,
    primers: &[Primer],
    options: &ScanOptions,
) -> Result<FileScanResult> {
    let mut reader = open_reader(reference)?;
    let file_name = reference.display().to_string();
    let mut line = String::new();
    let mut contig_name: Option<String> = None;
    let mut sequence = String::new();
    let mut collected_hits = Vec::new();
    let mut summary_acc = vec![SummaryAccumulator::default(); primers.len()];
    let mut total_hits = 0u64;

    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .with_context(|| format!("failed reading reference '{}'", reference.display()))?
            == 0
        {
            break;
        }

        let trimmed = line.trim_end_matches(['\n', '\r']).trim();
        if let Some(header) = trimmed.strip_prefix('>') {
            if let Some(current_contig) = contig_name.take() {
                let contig_result =
                    scan_contig(&file_name, &current_contig, &sequence, primers, options)?;
                total_hits += contig_result.total_hits;
                collected_hits.extend(contig_result.hits);
                for (acc, delta) in summary_acc
                    .iter_mut()
                    .zip(contig_result.summary.into_iter())
                {
                    acc.total_hits += delta.total_hits;
                    acc.perfect_hits += delta.perfect_hits;
                    acc.forward_hits += delta.forward_hits;
                    acc.reverse_hits += delta.reverse_hits;
                    acc.contigs_with_hits += delta.contigs_with_hits;
                }
                sequence.clear();
            }
            contig_name = Some(parse_contig_name(header));
        } else if !trimmed.is_empty() {
            if contig_name.is_none() {
                bail!(
                    "invalid FASTA '{}': found sequence before header",
                    reference.display()
                );
            }
            sequence.push_str(trimmed);
        }
    }

    if let Some(current_contig) = contig_name {
        let contig_result = scan_contig(&file_name, &current_contig, &sequence, primers, options)?;
        total_hits += contig_result.total_hits;
        collected_hits.extend(contig_result.hits);
        for (acc, delta) in summary_acc
            .iter_mut()
            .zip(contig_result.summary.into_iter())
        {
            acc.total_hits += delta.total_hits;
            acc.perfect_hits += delta.perfect_hits;
            acc.forward_hits += delta.forward_hits;
            acc.reverse_hits += delta.reverse_hits;
            acc.contigs_with_hits += delta.contigs_with_hits;
        }
    }

    Ok(FileScanResult {
        hits: collected_hits,
        summary: summary_acc,
        total_hits,
    })
}

fn scan_contig(
    file_name: &str,
    contig_name: &str,
    sequence: &str,
    primers: &[Primer],
    options: &ScanOptions,
) -> Result<ContigScanResult> {
    let sequence_bytes: Vec<u8> = sequence.bytes().map(normalize_base).collect();
    let sequence_masks: Vec<u8> = sequence_bytes
        .iter()
        .copied()
        .map(mask_or_unknown)
        .collect();

    if sequence_bytes.is_empty() {
        return Ok(ContigScanResult {
            hits: Vec::new(),
            summary: vec![SummaryAccumulator::default(); primers.len()],
            total_hits: 0,
        });
    }

    let per_primer = primers
        .par_iter()
        .enumerate()
        .map(|(idx, primer)| {
            scan_primer_in_contig(
                file_name,
                contig_name,
                &sequence_bytes,
                &sequence_masks,
                primer,
                idx,
                options,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let mut hits = Vec::new();
    let mut summary = vec![SummaryAccumulator::default(); primers.len()];
    let mut total_hits = 0u64;

    for primer_result in per_primer {
        total_hits += primer_result.summary.total_hits;
        summary[primer_result.primer_index] = primer_result.summary;
        hits.extend(primer_result.hits);
    }

    Ok(ContigScanResult {
        hits,
        summary,
        total_hits,
    })
}

fn scan_primer_in_contig(
    file_name: &str,
    contig_name: &str,
    sequence_bytes: &[u8],
    sequence_masks: &[u8],
    primer: &Primer,
    primer_index: usize,
    options: &ScanOptions,
) -> Result<PerPrimerContigResult> {
    if primer.is_empty() {
        bail!("primer '{}' has zero length", primer.name);
    }
    if sequence_bytes.len() < primer.len() {
        return Ok(PerPrimerContigResult {
            primer_index,
            hits: Vec::new(),
            summary: SummaryAccumulator::default(),
        });
    }

    let mut summary = SummaryAccumulator::default();
    let mut hits = Vec::new();

    scan_orientation(
        sequence_bytes,
        sequence_masks,
        primer,
        &primer.masks,
        '+',
        options.max_mismatches,
        file_name,
        contig_name,
        &mut summary,
        &mut hits,
    );

    if options.scan_reverse_complement && !primer.is_palindromic {
        scan_orientation(
            sequence_bytes,
            sequence_masks,
            primer,
            &primer.reverse_masks,
            '-',
            options.max_mismatches,
            file_name,
            contig_name,
            &mut summary,
            &mut hits,
        );
    }

    if summary.total_hits > 0 {
        summary.contigs_with_hits = 1;
    }

    Ok(PerPrimerContigResult {
        primer_index,
        hits,
        summary,
    })
}

#[allow(clippy::too_many_arguments)]
fn scan_orientation(
    sequence_bytes: &[u8],
    sequence_masks: &[u8],
    primer: &Primer,
    query_masks: &[u8],
    strand: char,
    max_mismatches: usize,
    file_name: &str,
    contig_name: &str,
    summary: &mut SummaryAccumulator,
    hits: &mut Vec<Hit>,
) {
    let window_len = query_masks.len();
    let last_start = sequence_masks.len() - window_len;

    for start in 0..=last_start {
        let mut mismatches = 0usize;
        for (offset, &query_mask) in query_masks.iter().enumerate() {
            if (query_mask & sequence_masks[start + offset]) == 0 {
                mismatches += 1;
                if mismatches > max_mismatches {
                    break;
                }
            }
        }

        if mismatches <= max_mismatches {
            summary.total_hits += 1;
            if mismatches == 0 {
                summary.perfect_hits += 1;
            }
            if strand == '+' {
                summary.forward_hits += 1;
            } else {
                summary.reverse_hits += 1;
            }

            hits.push(Hit {
                file: file_name.to_string(),
                contig: contig_name.to_string(),
                primer: primer.name.clone(),
                primer_len: primer.len(),
                start,
                end: start + primer.len(),
                strand,
                mismatches,
                matched: String::from_utf8_lossy(&sequence_bytes[start..start + primer.len()])
                    .to_string(),
            });
        }
    }
}

#[derive(Debug, Default, Clone)]
struct SummaryAccumulator {
    total_hits: u64,
    perfect_hits: u64,
    forward_hits: u64,
    reverse_hits: u64,
    contigs_with_hits: u64,
}

#[derive(Debug)]
struct FileScanResult {
    hits: Vec<Hit>,
    summary: Vec<SummaryAccumulator>,
    total_hits: u64,
}

#[derive(Debug)]
struct ContigScanResult {
    hits: Vec<Hit>,
    summary: Vec<SummaryAccumulator>,
    total_hits: u64,
}

#[derive(Debug)]
struct PerPrimerContigResult {
    primer_index: usize,
    hits: Vec<Hit>,
    summary: SummaryAccumulator,
}

fn parse_contig_name(header: &str) -> String {
    header
        .split_whitespace()
        .next()
        .filter(|x| !x.is_empty())
        .unwrap_or("unknown_contig")
        .to_string()
}

fn open_reader(path: &Path) -> Result<Box<dyn BufRead + Send>> {
    let file =
        File::open(path).with_context(|| format!("failed to open input '{}'", path.display()))?;
    let is_gz = path
        .extension()
        .and_then(|x| x.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gz"))
        .unwrap_or(false);

    if is_gz {
        Ok(Box::new(BufReader::new(MultiGzDecoder::new(file))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

fn infer_delimiter(line: &str) -> char {
    if line.contains('\t') { '\t' } else { ',' }
}

fn is_header(name: &str, sequence: &str) -> bool {
    let left = name.to_ascii_lowercase();
    let right = sequence.to_ascii_lowercase();
    (left == "name" || left == "primer" || left == "id")
        && (right == "sequence" || right == "primer" || right == "seq")
}

fn normalize_query(raw: &str) -> Result<String> {
    let mut normalized = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_whitespace() {
            continue;
        }
        let c = normalize_base(ch as u8) as char;
        if iupac_mask(c as u8).is_none() {
            bail!("unsupported base '{ch}' in primer sequence");
        }
        normalized.push(c);
    }
    Ok(normalized)
}

fn reverse_complement(sequence: &str) -> Result<String> {
    let mut out = String::with_capacity(sequence.len());
    for ch in sequence.bytes().rev() {
        let comp = complement_base(ch)
            .with_context(|| format!("unsupported base '{}' for reverse complement", ch as char))?;
        out.push(comp as char);
    }
    Ok(out)
}

fn to_masks(sequence: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(sequence.len());
    for ch in sequence.bytes() {
        out.push(
            iupac_mask(ch)
                .with_context(|| format!("unsupported base '{}' in primer", ch as char))?,
        );
    }
    Ok(out)
}

fn normalize_base(base: u8) -> u8 {
    match base {
        b'u' | b'U' => b'T',
        _ => base.to_ascii_uppercase(),
    }
}

fn mask_or_unknown(base: u8) -> u8 {
    iupac_mask(base).unwrap_or(0b1111)
}

fn complement_base(base: u8) -> Option<u8> {
    match normalize_base(base) {
        b'A' => Some(b'T'),
        b'C' => Some(b'G'),
        b'G' => Some(b'C'),
        b'T' => Some(b'A'),
        b'R' => Some(b'Y'),
        b'Y' => Some(b'R'),
        b'S' => Some(b'S'),
        b'W' => Some(b'W'),
        b'K' => Some(b'M'),
        b'M' => Some(b'K'),
        b'B' => Some(b'V'),
        b'D' => Some(b'H'),
        b'H' => Some(b'D'),
        b'V' => Some(b'B'),
        b'N' => Some(b'N'),
        _ => None,
    }
}

fn iupac_mask(base: u8) -> Option<u8> {
    match normalize_base(base) {
        b'A' => Some(0b0001),
        b'C' => Some(0b0010),
        b'G' => Some(0b0100),
        b'T' => Some(0b1000),
        b'R' => Some(0b0101),
        b'Y' => Some(0b1010),
        b'S' => Some(0b0110),
        b'W' => Some(0b1001),
        b'K' => Some(0b1100),
        b'M' => Some(0b0011),
        b'B' => Some(0b1110),
        b'D' => Some(0b1101),
        b'H' => Some(0b1011),
        b'V' => Some(0b0111),
        b'N' => Some(0b1111),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("primer_scout_{nanos}_{name}"))
    }

    #[test]
    fn reverse_complement_handles_iupac() {
        let rc = reverse_complement("ATGCRY").expect("reverse complement should work");
        assert_eq!(rc, "RYGCAT");
    }

    #[test]
    fn load_primers_with_header_and_tab() {
        let file = tmp_path("primers.tsv");
        {
            let mut f = std::fs::File::create(&file).expect("create file");
            writeln!(f, "name\tsequence").expect("write header");
            writeln!(f, "p1\tATGC").expect("write primer p1");
            writeln!(f, "p2\tTTRA").expect("write primer p2");
        }
        let primers = load_primers(&file).expect("load primers");
        assert_eq!(primers.len(), 2);
        assert_eq!(primers[0].name, "p1");
        assert_eq!(primers[0].sequence, "ATGC");
        assert_eq!(primers[1].reverse_complement, "TYAA");
        std::fs::remove_file(file).expect("remove tmp file");
    }

    #[test]
    fn scan_finds_forward_and_reverse_hits() {
        let reference = tmp_path("ref.fa");
        let primers_file = tmp_path("primers.tsv");
        {
            let mut rf = std::fs::File::create(&reference).expect("create reference");
            writeln!(rf, ">chr1").expect("write header");
            writeln!(rf, "TTTATGCCCGGCATTT").expect("write sequence");
        }
        {
            let mut pf = std::fs::File::create(&primers_file).expect("create primers");
            writeln!(pf, "name\tsequence").expect("write header");
            writeln!(pf, "p1\tATGC").expect("write primer");
        }

        let primers = load_primers(&primers_file).expect("load primers");
        let result = scan_references(
            std::slice::from_ref(&reference),
            &primers,
            &ScanOptions {
                max_mismatches: 0,
                scan_reverse_complement: true,
            },
        )
        .expect("scan references");

        assert_eq!(result.total_hits, 2);
        assert_eq!(result.hits.len(), 2);
        let forward = result
            .hits
            .iter()
            .find(|h| h.strand == '+')
            .expect("forward hit");
        assert_eq!(forward.start, 3);
        let reverse = result
            .hits
            .iter()
            .find(|h| h.strand == '-')
            .expect("reverse hit");
        assert_eq!(reverse.start, 10);

        std::fs::remove_file(reference).expect("remove ref");
        std::fs::remove_file(primers_file).expect("remove primers");
    }

    #[test]
    fn mismatch_threshold_is_respected() {
        let primer = Primer {
            name: "p".to_string(),
            sequence: "ATGC".to_string(),
            reverse_complement: "GCAT".to_string(),
            masks: vec![0b0001, 0b1000, 0b0100, 0b0010],
            reverse_masks: vec![0b0100, 0b0010, 0b0001, 0b1000],
            is_palindromic: false,
        };

        let result = scan_contig(
            "ref.fa",
            "chr1",
            "ATGT",
            &[primer],
            &ScanOptions {
                max_mismatches: 1,
                scan_reverse_complement: false,
            },
        )
        .expect("scan contig");

        assert_eq!(result.total_hits, 1);
        assert_eq!(result.hits[0].mismatches, 1);
    }
}
