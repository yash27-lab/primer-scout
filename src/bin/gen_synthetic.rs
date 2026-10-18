#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fasta_wrapping() {
        let path = fixture();
        write_fasta(&path, "test", &vec![b'A'; 81]).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines[0], ">test");
        assert_eq!(lines[1].len(), 80);
        assert_eq!(lines[2], "A");
    }

    #[test]
    fn mutate_t() {
        let mut r = XorShift64::new(42);
        for _ in 0..100 {
            let b = mutate_base(b'T', &mut r);
            assert_ne!(b, b'T');
            assert!(b"ACGT".contains(&b));
        }
    }

    #[test]
    fn mutate_g() {
        let mut r = XorShift64::new(42);
        for _ in 0..100 {
            let b = mutate_base(b'G', &mut r);
            assert_ne!(b, b'G');
            assert!(b"ACGT".contains(&b));
        }
    }

    #[test]
    fn mutate_c() {
        let mut r = XorShift64::new(42);
        for _ in 0..100 {
            let b = mutate_base(b'C', &mut r);
            assert_ne!(b, b'C');
            assert!(b"ACGT".contains(&b));
        }
    }

    #[test]
    fn mutate_a() {
        let mut r = XorShift64::new(42);
        for _ in 0..100 {
            let b = mutate_base(b'A', &mut r);
            assert_ne!(b, b'A');
            assert!(b"ACGT".contains(&b));
        }
    }

    #[test]
    fn empty_rng() {
        let mut a = XorShift64::new(42);
        generate_sequence(0, &mut a);
        assert_eq!(a.next_u32(), XorShift64::new(42).next_u32());
    }

    #[test]
    fn cloned_rng() {
        let mut a = XorShift64::new(42);
        a.next_u32();
        let mut b = a.clone();
        assert_eq!(a.next_u32(), b.next_u32());
    }

    #[test]
    fn zero_seed() {
        let a = generate_sequence(100, &mut XorShift64::new(0));
        assert_eq!(a, generate_sequence(100, &mut XorShift64::new(0)));
        assert!(a.windows(2).any(|w| w[0] != w[1]));
    }

    #[test]
    fn seed_variation() {
        assert_ne!(
            generate_sequence(100, &mut XorShift64::new(1)),
            generate_sequence(100, &mut XorShift64::new(2))
        );
    }

    #[test]
    fn sequence_repeatability() {
        assert_eq!(
            generate_sequence(100, &mut XorShift64::new(42)),
            generate_sequence(100, &mut XorShift64::new(42))
        );
    }

    #[test]
    fn sequence_alphabet() {
        assert!(
            generate_sequence(1000, &mut XorShift64::new(42))
                .iter()
                .all(|b| b"ACGT".contains(b))
        );
    }

    #[test]
    fn sequence_length() {
        assert_eq!(generate_sequence(257, &mut XorShift64::new(42)).len(), 257);
    }

    #[test]
    fn empty_sequence() {
        assert!(generate_sequence(0, &mut XorShift64::new(42)).is_empty());
    }

    fn fixture() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "primer_generator_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }
}

use anyhow::{Context, Result, bail};
use clap::Parser;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = Args::parse();
    if args.primer_len == 0 {
        bail!("--primer-len must be > 0");
    }
    if args.bases <= args.primer_len {
        bail!("--bases must be greater than --primer-len");
    }
    if args.primer_count == 0 {
        bail!("--primer-count must be > 0");
    }

    let mut rng = XorShift64::new(args.seed);
    let sequence = generate_sequence(args.bases, &mut rng);
    write_fasta(&args.reference_out, "synthetic_chr1", &sequence)?;
    write_primers(
        &args.primers_out,
        &sequence,
        args.primer_count,
        args.primer_len,
        &mut rng,
    )?;
    Ok(())
}

#[derive(Debug, Parser)]
#[command(
    name = "gen-synthetic",
    version,
    about = "Generate deterministic synthetic FASTA + primer panel for benchmarks"
)]
struct Args {
    #[arg(long, default_value = "benchmarks/generated/reference.fa")]
    reference_out: PathBuf,

    #[arg(long, default_value = "benchmarks/generated/primers.tsv")]
    primers_out: PathBuf,

    #[arg(long, default_value_t = 5_000_000)]
    bases: usize,

    #[arg(long, default_value_t = 128)]
    primer_count: usize,

    #[arg(long, default_value_t = 20)]
    primer_len: usize,

    #[arg(long, default_value_t = 42)]
    seed: u64,
}

fn generate_sequence(len: usize, rng: &mut XorShift64) -> Vec<u8> {
    const BASES: [u8; 4] = *b"ACGT";
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(BASES[(rng.next_u32() as usize) & 3]);
    }
    out
}

fn write_fasta(path: &PathBuf, contig_name: &str, sequence: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    }

    let file =
        File::create(path).with_context(|| format!("failed to create '{}'", path.display()))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, ">{contig_name}")?;
    for chunk in sequence.chunks(80) {
        writeln!(writer, "{}", String::from_utf8_lossy(chunk))?;
    }
    writer.flush()?;
    Ok(())
}

fn write_primers(
    path: &PathBuf,
    sequence: &[u8],
    primer_count: usize,
    primer_len: usize,
    rng: &mut XorShift64,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    }

    let file =
        File::create(path).with_context(|| format!("failed to create '{}'", path.display()))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "name\tsequence")?;

    let start_count = sequence.len() - primer_len + 1;
    for i in 0..primer_count {
        let start = (rng.next_u32() as usize) % start_count;
        let mut primer = sequence[start..start + primer_len].to_vec();

        // Every 5th primer gets one deterministic mismatch to simulate off-target tolerant usage.
        if i % 5 == 0 {
            let pos = (rng.next_u32() as usize) % primer_len;
            primer[pos] = mutate_base(primer[pos], rng);
        }

        writeln!(
            writer,
            "p{:04}\t{}",
            i + 1,
            String::from_utf8_lossy(&primer)
        )?;
    }

    writer.flush()?;
    Ok(())
}

fn mutate_base(current: u8, rng: &mut XorShift64) -> u8 {
    const BASES: [u8; 4] = *b"ACGT";
    for _ in 0..10 {
        let candidate = BASES[(rng.next_u32() as usize) & 3];
        if candidate != current {
            return candidate;
        }
    }
    match current {
        b'A' => b'C',
        b'C' => b'G',
        b'G' => b'T',
        _ => b'A',
    }
}

#[derive(Debug, Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xA5A5_A5A5_A5A5_A5A5
            } else {
                seed
            },
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }
}
