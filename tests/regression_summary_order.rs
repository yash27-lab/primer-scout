use primer_scout::{Primer, ScanOptions, scan_sequence};

#[test]
fn summary_order() {
    let p = [
        Primer::from_name_and_sequence("z", "AA").unwrap(),
        Primer::from_name_and_sequence("a", "CC").unwrap(),
    ];
    let r = scan_sequence("AACC", "chr", &p, &ScanOptions::default()).unwrap();
    assert_eq!(
        r.summary
            .iter()
            .map(|s| s.primer.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "z"]
    );
}
