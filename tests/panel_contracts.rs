use primer_scout::{Primer, load_primers};

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
