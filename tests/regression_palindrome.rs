use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn palindrome() {
    let r = scan_sequence(
        "ATAT",
        "chr",
        &[Primer::from_name_and_sequence("p", "ATAT").unwrap()],
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(r.total_hits, 1);
    assert_eq!(r.summary[0].reverse_hits, 0);
}
