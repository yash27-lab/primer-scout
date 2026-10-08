use primer_scout::{ScanOptions, scan_sequence};

#[test]
fn empty_panel() {
    assert!(scan_sequence("ACGT", "chr", &[], &ScanOptions::default()).is_err());
}
