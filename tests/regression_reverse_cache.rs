use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn reverse_cache() {
    let mut p = Primer::from_name_and_sequence("p", "ATGC").unwrap();
    p.reverse_complement = "AAAA".to_owned();
    assert!(p.validate().is_err());
    assert!(scan_sequence("ATGC", "chr", &[p], &ScanOptions::default()).is_err());
}
