use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn lowercase_reference() {
    let r = scan_sequence(
        "atgc",
        "chr",
        &[Primer::from_name_and_sequence("p", "ATGC").unwrap()],
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(r.total_hits, 1);
    assert_eq!(r.hits[0].mismatches, 0);
}
