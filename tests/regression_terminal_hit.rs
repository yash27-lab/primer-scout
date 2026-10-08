use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn terminal_hit() {
    let r = scan_sequence(
        "CCAT",
        "chr",
        &[Primer::from_name_and_sequence("p", "AT").unwrap()],
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(r.hits.len(), 1);
    assert_eq!((r.hits[0].start, r.hits[0].end), (2, 4));
}
