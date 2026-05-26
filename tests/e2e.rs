use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::tempdir;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn cmd() -> Command {
    let mut c = Command::cargo_bin("ctoken").unwrap();
    let config_dir = tempdir().unwrap();
    let path = config_dir.keep();
    c.env("CTOKEN_CONFIG_DIR", &path);
    c
}

// ── 1. Default mode ────────────────────────────────────────────────────────────

#[test]
fn default_mode_rust_project() {
    cmd()
        .arg(fixture("rust_project"))
        .assert()
        .success()
        .stdout(predicate::str::contains("DIRECTORY"))
        .stdout(predicate::str::contains("TOKENS"))
        .stdout(predicate::str::contains("TOTAL"));
}

// ── 2. -t mode ────────────────────────────────────────────────────────────────

#[test]
fn type_mode_rust_project() {
    cmd()
        .arg(fixture("rust_project"))
        .arg("-t")
        .assert()
        .success()
        .stdout(predicate::str::contains("TYPE"))
        .stdout(predicate::str::contains("FILES"))
        .stdout(predicate::str::contains("rs"));
}

// ── 3. -s sum ────────────────────────────────────────────────────────────────

#[test]
fn sum_mode_is_integer() {
    let output = cmd()
        .arg(fixture("rust_project"))
        .arg("-s")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    let n: usize = trimmed
        .parse()
        .expect("sum output should be a plain integer");
    assert!(n > 0, "sum should be positive");
}

// ── 4. --json ────────────────────────────────────────────────────────────────

#[test]
fn json_mode_parses() {
    let output = cmd()
        .arg(fixture("rust_project"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(v["total"].as_u64().unwrap() > 0);
    assert!(v["entries"].is_array());
}

// ── 5. -p rust over mixed/ ───────────────────────────────────────────────────

#[test]
fn profile_rust_excludes_python() {
    let output = cmd()
        .arg(fixture("mixed"))
        .arg("-p")
        .arg("rust")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let entries = v["entries"].as_array().unwrap();
    // no python entry
    assert!(!entries.iter().any(|e| e["path"] == "app.py"));
}

// ── 6. -g off includes gitignored files ──────────────────────────────────────

#[test]
fn gitignore_off_includes_log() {
    // Use a self-contained tempdir so we don't depend on fixture files that git
    // itself would refuse to track (they'd match the fixture's own .gitignore).
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Minimal .git dir so the ignore crate applies .gitignore rules.
    std::fs::create_dir(root.join(".git")).unwrap();
    std::fs::write(root.join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join("ignored.log"), "this log is gitignored\n").unwrap();

    let out_on = cmd()
        .arg(root)
        .arg("-g").arg("on").arg("-s")
        .output().unwrap();
    assert!(out_on.status.success());
    let n_on: usize = String::from_utf8_lossy(&out_on.stdout).trim().parse().unwrap();

    let out_off = cmd()
        .arg(root)
        .arg("-g").arg("off").arg("-s")
        .output().unwrap();
    assert!(out_off.status.success());
    let n_off: usize = String::from_utf8_lossy(&out_off.stdout).trim().parse().unwrap();

    assert!(n_off > n_on, "gitignore=off should count more tokens than gitignore=on");
}

// ── 7. -m glob filters to matching files ────────────────────────────────────

#[test]
fn match_glob_md_only() {
    let output = cmd()
        .arg(fixture("rust_project"))
        .arg("-m")
        .arg("**/*.md")
        .arg("-t")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("md"), "should have md extension in output");
    assert!(!stdout.contains(" rs "), "should not have rs extension");
}

// ── 8. --recursive produces per-dir blocks ──────────────────────────────────

#[test]
fn recursive_per_dir_blocks() {
    cmd()
        .arg(fixture("rust_project"))
        .arg("--recursive")
        .assert()
        .success()
        .stdout(predicate::str::contains("==="))
        .stdout(predicate::str::contains("GRAND TOTAL"));
}

// ── 9. --recursive + --json errors out ──────────────────────────────────────

#[test]
fn recursive_json_errors() {
    cmd()
        .arg(fixture("rust_project"))
        .arg("--recursive")
        .arg("--json")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--json"));
}

// ── 10. Binary file skipped, logged with -v ──────────────────────────────────

#[test]
fn binary_file_skipped() {
    let output = cmd()
        .arg(fixture("binary_assets"))
        .arg("-s")
        .output()
        .unwrap();
    assert!(output.status.success());
    let total: usize = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap();
    // plain.txt should contribute tokens; png and bin should not
    // "this is plain text\n" is at least 1 token
    assert!(total > 0);

    // with -v, binary files logged
    cmd()
        .arg(fixture("binary_assets"))
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("(binary)"));
}

// ── 11. -v produces one line per file ────────────────────────────────────────

#[test]
fn verbose_one_line_per_file() {
    let output = cmd()
        .arg(fixture("rust_project"))
        .arg("-g")
        .arg("on")
        .arg("-v")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // every line before the table contains " - "
    let verbose_lines: Vec<_> = stdout
        .lines()
        .filter(|l| l.contains(" - ") && !l.contains("DIRECTORY") && !l.contains("TOTAL"))
        .collect();
    assert!(!verbose_lines.is_empty());
    for line in &verbose_lines {
        // line format: padded number or spaces, then " - ", then path
        assert!(line.contains(" - "), "each verbose line must contain ' - '");
    }
}

// ── 12. Missing path argument → exit 2 ──────────────────────────────────────

#[test]
fn no_path_arg_exits_2() {
    cmd().assert().failure().code(2);
}

// ── 13. Unknown encoding → exit 2 ────────────────────────────────────────────

#[test]
fn unknown_encoding_exits_2() {
    cmd()
        .arg(fixture("rust_project"))
        .arg("--encoding")
        .arg("bogus_encoding_xyz")
        .assert()
        .failure()
        .code(2);
}

// ── 14. Single-file input: prints integer ───────────────────────────────────

#[test]
fn single_file_prints_integer() {
    let file = fixture("rust_project").join("src").join("main.rs");
    let output = cmd().arg(&file).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let n: usize = stdout
        .trim()
        .parse()
        .expect("single file output should be an integer");
    assert!(n > 0);
}

// ── 15. Single-file + --json → {"path": ..., "tokens": N} ──────────────────

#[test]
fn single_file_json() {
    let file = fixture("rust_project").join("src").join("main.rs");
    let output = cmd().arg(&file).arg("--json").output().unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["tokens"].as_u64().unwrap() > 0);
    assert!(v["path"].as_str().unwrap().ends_with("main.rs"));
}

// ── 16. Single-file ignores -t / --recursive / -p ──────────────────────────

#[test]
fn single_file_ignores_dir_flags() {
    let file = fixture("rust_project").join("src").join("main.rs");
    // -t and --recursive should be silently ignored; output is still an integer
    let output = cmd()
        .arg(&file)
        .arg("-t")
        .arg("-p")
        .arg("rust")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _n: usize = stdout.trim().parse().expect("still outputs an integer");
}

// ── 17. Nonexistent path → exit 2 ────────────────────────────────────────────

#[test]
fn nonexistent_path_exits_2() {
    cmd()
        .arg("/this/path/does/not/exist/xyz")
        .assert()
        .failure()
        .code(2);
}

// ── 18. -s + --json → {"total": N} ──────────────────────────────────────────

#[test]
fn sum_json_shape() {
    let output = cmd()
        .arg(fixture("rust_project"))
        .arg("-s")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["total"].as_u64().unwrap() > 0);
    assert!(
        v.as_object().unwrap().len() == 1,
        "should only have 'total' key"
    );
}

// ── 19. Unknown profile → exit 2 with available list ────────────────────────

#[test]
fn unknown_profile_exits_2() {
    cmd()
        .arg(fixture("rust_project"))
        .arg("-p")
        .arg("nonexistent_profile_xyz")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

// ── 20. --recursive-with-dir includes child dir counts ──────────────────────

#[test]
fn recursive_with_dir_mode() {
    cmd()
        .arg(fixture("rust_project"))
        .arg("--recursive-with-dir")
        .assert()
        .success()
        .stdout(predicate::str::contains("==="))
        .stdout(predicate::str::contains("GRAND TOTAL"));
}
