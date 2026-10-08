use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn empty_panel() {
    assert!(scan_sequence("ACGT", "chr", &[], &ScanOptions::default()).is_err());
}
