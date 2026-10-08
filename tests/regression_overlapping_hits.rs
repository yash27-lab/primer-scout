use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn overlapping_hits() {
    let r = scan_sequence(
        "AAAA",
        "chr",
        &[Primer::from_name_and_sequence("p", "AA").unwrap()],
        &ScanOptions {
            max_mismatches: 0,
            scan_reverse_complement: false,
        },
    )
    .unwrap();
    assert_eq!(
        r.hits.iter().map(|h| h.start).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}
