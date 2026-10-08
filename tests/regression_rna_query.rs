use primer_scout::Primer;

#[test]
fn rna_query() {
    let p = Primer::from_name_and_sequence("rna", "auGc").unwrap();
    assert_eq!(p.sequence, "ATGC");
    assert_eq!(p.reverse_complement, "GCAT");
    p.validate().unwrap();
}
