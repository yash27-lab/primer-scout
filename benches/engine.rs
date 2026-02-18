use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use primer_scout::{Primer, ScanOptions, scan_sequence};
use std::hint::black_box;

fn benchmark_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan_sequence");
    let sequence_len = 1_000_000usize;
    let primer_len = 20usize;
    let primer_counts = [32usize, 128usize];

    let sequence = generate_sequence(sequence_len, 7);
    group.throughput(Throughput::Bytes(sequence_len as u64));

    for &count in &primer_counts {
        let primers = generate_primers_from_reference(&sequence, count, primer_len);
        for &k in &[0usize, 1usize] {
            let options = ScanOptions {
                max_mismatches: k,
                scan_reverse_complement: true,
            };
            group.bench_with_input(
                BenchmarkId::new(format!("primers_{count}"), format!("k{k}")),
                &options,
                |b, opts| {
                    b.iter_batched(
                        || (sequence.clone(), primers.clone()),
                        |(seq, panel)| {
                            let res =
                                scan_sequence(&seq, "synthetic_chr1", &panel, opts).expect("scan");
                            black_box(res.total_hits);
                        },
                        BatchSize::SmallInput,
                    );
                },
            );
        }
    }
    group.finish();
}

fn generate_sequence(len: usize, seed: u64) -> String {
    const BASES: [u8; 4] = [b'A', b'C', b'G', b'T'];
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(BASES[(rng.next_u32() as usize) & 3]);
    }
    String::from_utf8(out).expect("bases are valid ASCII")
}

fn generate_primers_from_reference(
    reference: &str,
    count: usize,
    primer_len: usize,
) -> Vec<Primer> {
    let mut rng = XorShift64::new(11);
    let bytes = reference.as_bytes();
    let max_start = bytes.len() - primer_len;
    let mut out = Vec::with_capacity(count);

    for idx in 0..count {
        let start = (rng.next_u32() as usize) % max_start;
        let mut seq = bytes[start..start + primer_len].to_vec();
        if idx % 4 == 0 {
            let pos = (rng.next_u32() as usize) % primer_len;
            seq[pos] = mutate_base(seq[pos], &mut rng);
        }
        out.push(
            Primer::from_name_and_sequence(
                format!("p{:04}", idx + 1),
                &String::from_utf8_lossy(&seq),
            )
            .expect("primer should be valid"),
        );
    }
    out
}

fn mutate_base(current: u8, rng: &mut XorShift64) -> u8 {
    const BASES: [u8; 4] = [b'A', b'C', b'G', b'T'];
    for _ in 0..8 {
        let cand = BASES[(rng.next_u32() as usize) & 3];
        if cand != current {
            return cand;
        }
    }
    b'A'
}

#[derive(Debug, Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
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

criterion_group!(benches, benchmark_engine);
criterion_main!(benches);
