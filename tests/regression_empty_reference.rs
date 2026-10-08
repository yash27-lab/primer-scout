use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn empty_reference() {
    let r = scan_sequence(
        "",
        "chr",
        &[Primer::from_name_and_sequence("p", "AAA").unwrap()],
        &ScanOptions::default(),
    )
    .unwrap();
    assert!(r.hits.is_empty());
    assert_eq!(r.summary.len(), 1);
    assert_eq!(r.summary[0].total_hits, 0);
}
