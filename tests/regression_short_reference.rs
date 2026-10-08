use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn short_reference() {
    let r = scan_sequence(
        "A",
        "chr",
        &[Primer::from_name_and_sequence("p", "AAA").unwrap()],
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(r.total_hits, 0);
    assert_eq!(r.summary[0].contigs_with_hits, 0);
}
