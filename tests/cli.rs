//! Black-box integration tests that drive the compiled `primer-scout` binary
//! end-to-end against the bundled demo data and small temporary fixtures.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// Path to the binary under test, provided by Cargo for the integration target.
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_primer-scout")
}

/// Build a unique temporary path, mirroring the `SystemTime` nanos style used by
/// the unit tests in `src/lib.rs`. The process id is folded in so concurrent
/// test binaries cannot collide on the same name.
fn tmp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("primer_scout_it_{pid}_{nanos}_{name}"))
}

/// Run the binary with the given arguments and capture its output.
fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("primer-scout binary should spawn")
}

fn stdout_string(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be valid UTF-8")
}

fn stderr_string(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be valid UTF-8")
}

#[test]
fn count_only_demo_data() {
    let output = run(&[
        "--primers",
        "data/demo_primers.tsv",
        "--reference",
        "data/demo.fa",
        "--count-only",
    ]);
    assert!(
        output.status.success(),
        "count-only scan should succeed, stderr: {}",
        stderr_string(&output)
    );
    assert_eq!(stdout_string(&output).trim(), "27");
}

#[test]
fn summary_has_expected_rows() {
    let output = run(&[
        "--primers",
        "data/demo_primers.tsv",
        "--reference",
        "data/demo.fa",
        "--summary",
    ]);
    assert!(
        output.status.success(),
        "summary scan should succeed, stderr: {}",
        stderr_string(&output)
    );
    let stdout = stdout_string(&output);

    for primer in ["p_atgc", "p_gatc", "p_ambig"] {
        assert!(
            stdout
                .lines()
                .any(|line| { line.split('\t').next().map(str::trim) == Some(primer) }),
            "summary should contain a row for {primer}, got:\n{stdout}"
        );
    }

    // The `p_atgc` row is `primer<tab>primer_len<tab>total_hits<tab>...`; its
    // total_hits column (index 2) is 12 for the demo data.
    let atgc_row = stdout
        .lines()
        .find(|line| line.split('\t').next().map(str::trim) == Some("p_atgc"))
        .expect("p_atgc row should exist");
    let fields: Vec<&str> = atgc_row.split('\t').map(str::trim).collect();
    assert_eq!(fields[0], "p_atgc");
    assert_eq!(fields[2], "12", "p_atgc total_hits should be 12");
}

#[test]
fn json_count_only_is_valid_ndjson() {
    let output = run(&[
        "--primers",
        "data/demo_primers.tsv",
        "--reference",
        "data/demo.fa",
        "--count-only",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "json count-only scan should succeed, stderr: {}",
        stderr_string(&output)
    );
    let stdout = stdout_string(&output);
    // Exactly one NDJSON line (trailing newline aside).
    assert_eq!(stdout.trim().lines().count(), 1, "expected one JSON line");
    let line = stdout.trim();
    assert!(
        line.contains("\"total_hits\""),
        "JSON should carry a total_hits field, got: {line}"
    );
    assert!(line.contains("27"), "JSON should report 27, got: {line}");
}

#[test]
fn csv_primer_input_parses() {
    let primers = tmp_path("primers.csv");
    {
        let mut f = std::fs::File::create(&primers).expect("create csv primer file");
        writeln!(f, "name,sequence").expect("write csv header");
        writeln!(f, "p_atgc,ATGC").expect("write csv primer row");
    }

    let output = run(&[
        "--primers",
        primers.to_str().expect("csv path is valid UTF-8"),
        "--reference",
        "data/demo.fa",
        "--count-only",
    ]);

    let success = output.status.success();
    let stdout = stdout_string(&output);
    let stderr = stderr_string(&output);
    std::fs::remove_file(&primers).expect("remove tmp csv primer file");

    assert!(
        success,
        "csv count-only scan should succeed, stderr: {stderr}"
    );
    let trimmed = stdout.trim();
    let count: u64 = trimmed
        .parse()
        .unwrap_or_else(|_| panic!("expected an integer count, got: {trimmed:?}"));
    assert!(count > 0, "ATGC primer should match the demo reference");
}

#[test]
fn gzip_reference_matches_plain() {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    // Count against the plain reference first.
    let plain = run(&[
        "--primers",
        "data/demo_primers.tsv",
        "--reference",
        "data/demo.fa",
        "--count-only",
    ]);
    assert!(
        plain.status.success(),
        "plain count-only scan should succeed, stderr: {}",
        stderr_string(&plain)
    );
    let plain_count = stdout_string(&plain).trim().to_string();

    // Gzip the same FASTA bytes into a temporary `.fa.gz`.
    let gz_path = tmp_path("demo.fa.gz");
    let raw = std::fs::read("data/demo.fa").expect("read demo.fa");
    {
        let file = std::fs::File::create(&gz_path).expect("create gz reference");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(&raw).expect("write gzip body");
        encoder.finish().expect("finish gzip stream");
    }

    let gz = run(&[
        "--primers",
        "data/demo_primers.tsv",
        "--reference",
        gz_path.to_str().expect("gz path is valid UTF-8"),
        "--count-only",
    ]);

    let gz_success = gz.status.success();
    let gz_count = stdout_string(&gz).trim().to_string();
    let gz_stderr = stderr_string(&gz);
    std::fs::remove_file(&gz_path).expect("remove tmp gz reference");

    assert!(
        gz_success,
        "gzip count-only scan should succeed, stderr: {gz_stderr}"
    );
    assert_eq!(
        gz_count, plain_count,
        "gzip reference count should match the plain reference count"
    );
}

#[test]
fn no_revcomp_reduces_or_equals_hits() {
    let default = run(&[
        "--primers",
        "data/demo_primers.tsv",
        "--reference",
        "data/demo.fa",
        "--count-only",
    ]);
    assert!(
        default.status.success(),
        "default count-only scan should succeed, stderr: {}",
        stderr_string(&default)
    );
    let default_count: u64 = stdout_string(&default)
        .trim()
        .parse()
        .expect("default count should be an integer");

    let no_rc = run(&[
        "--primers",
        "data/demo_primers.tsv",
        "--reference",
        "data/demo.fa",
        "--count-only",
        "--no-revcomp",
    ]);
    assert!(
        no_rc.status.success(),
        "--no-revcomp count-only scan should succeed, stderr: {}",
        stderr_string(&no_rc)
    );
    let no_rc_count: u64 = stdout_string(&no_rc)
        .trim()
        .parse()
        .expect("--no-revcomp count should be an integer");

    assert!(
        no_rc_count <= default_count,
        "--no-revcomp total ({no_rc_count}) must not exceed default total ({default_count})"
    );
    // The demo reference contains reverse-strand hits, so dropping them must
    // strictly reduce the total.
    assert!(
        no_rc_count < default_count,
        "--no-revcomp total ({no_rc_count}) should be strictly less than default ({default_count}) for the demo set"
    );
}

#[test]
fn missing_primer_file_fails() {
    let output = run(&[
        "--primers",
        "/nonexistent/path.tsv",
        "--reference",
        "data/demo.fa",
        "--count-only",
    ]);
    assert!(
        !output.status.success(),
        "scan against a missing primer file should fail"
    );
}

#[test]
fn empty_primer_file_fails() {
    let primers = tmp_path("empty_primers.tsv");
    std::fs::File::create(&primers).expect("create empty primer file");

    let output = run(&[
        "--primers",
        primers.to_str().expect("primer path is valid UTF-8"),
        "--reference",
        "data/demo.fa",
        "--count-only",
    ]);

    let failed = !output.status.success();
    std::fs::remove_file(&primers).expect("remove tmp empty primer file");
    assert!(failed, "scan against an empty primer file should fail");
}

#[test]
fn invalid_base_fails() {
    let primers = tmp_path("invalid_base.tsv");
    {
        let mut f = std::fs::File::create(&primers).expect("create primer file");
        writeln!(f, "p1\tATXZ").expect("write primer with illegal base");
    }

    let output = run(&[
        "--primers",
        primers.to_str().expect("primer path is valid UTF-8"),
        "--reference",
        "data/demo.fa",
        "--count-only",
    ]);

    let failed = !output.status.success();
    std::fs::remove_file(&primers).expect("remove tmp primer file");
    assert!(failed, "scan with an illegal primer base should fail");
}

#[test]
fn contig_base_limit_enforced() {
    let output = Command::new(bin())
        .args([
            "--primers",
            "data/demo_primers.tsv",
            "--reference",
            "data/demo.fa",
            "--count-only",
        ])
        .env("PRIMER_SCOUT_MAX_CONTIG_BASES", "1")
        .output()
        .expect("primer-scout binary should spawn");

    assert!(
        !output.status.success(),
        "scan should fail when the contig base limit is exceeded"
    );
    let stderr = stderr_string(&output);
    assert!(
        stderr.contains("PRIMER_SCOUT_MAX_CONTIG_BASES") || stderr.contains("safety limit"),
        "stderr should mention the contig base limit, got: {stderr}"
    );
}
