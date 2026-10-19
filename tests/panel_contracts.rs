use primer_scout::{Primer, load_primers};

#[test]
fn generated_collision() {
    assert!(panel("primer_0002\tATGC\n\tGCTA\n").is_err());
}

#[test]
fn comments_only() {
    let e = panel("# note\n\n").unwrap_err();
    assert!(e.to_string().contains("no primers found"));
}

#[test]
fn header_only() {
    assert!(panel("name\tsequence\n").is_err());
}

#[test]
fn interleaved_comments() {
    let p = panel("ATGC\n# note\n\nGCTA\n").unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[1].name, "primer_0002");
}

#[test]
fn leading_comments() {
    assert_eq!(
        panel("# note\n\nname\tsequence\nalpha\tATGC\n").unwrap()[0].name,
        "alpha"
    );
}

#[test]
fn no_final_newline() {
    assert_eq!(panel("alpha\tATGC").unwrap()[0].sequence, "ATGC");
}

#[test]
fn crlf() {
    let p = panel("name\tsequence\r\nalpha\tATGC\r\n").unwrap();
    assert_eq!(p[0].sequence, "ATGC");
}

#[test]
fn header_case() {
    assert_eq!(panel("NAME\tSEQUENCE\nalpha\tATGC\n").unwrap().len(), 1);
}

#[test]
fn header_aliases() {
    for h in ["name\tsequence", "primer\tseq", "id\tprimer"] {
        let p = panel(&format!("{h}\nalpha\tATGC\n")).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "alpha");
    }
}

#[test]
fn csv_without_header() {
    let p = panel("alpha,ATGC\nbeta,GCTA\n").unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[1].name, "beta");
}

#[test]
fn sequence_only() {
    let p = panel("ATGC\nGCTA\n").unwrap();
    assert_eq!(p[0].name, "primer_0001");
    assert_eq!(p[1].name, "primer_0002");
}

fn panel(text: &str) -> anyhow::Result<Vec<Primer>> {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "primer_panel_contract_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, text)?;
    let result = load_primers(&path);
    std::fs::remove_file(path)?;
    result
}
