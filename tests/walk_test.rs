use std::fs;
use std::path::Path;
use std::process::Command;

fn repo39_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repo39"))
}

fn create_tree(base: &Path) {
    fs::create_dir_all(base.join("src/nested")).unwrap();
    fs::create_dir_all(base.join(".hidden_dir")).unwrap();
    fs::create_dir_all(base.join("node_modules/pkg")).unwrap();
    fs::write(base.join("readme.txt"), "hello").unwrap();
    fs::write(base.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(base.join("src/nested/lib.rs"), "// lib").unwrap();
    fs::write(base.join(".secret"), "shh").unwrap();
    fs::write(base.join(".hidden_dir/data"), "x").unwrap();
    fs::write(base.join("node_modules/pkg/index.js"), "//").unwrap();
}

fn run_on_tree(show: Option<&str>) -> (tempfile::TempDir, Vec<String>) {
    let tmp = tempfile::tempdir().unwrap();
    create_tree(tmp.path());
    let mut args = vec![tmp.path().to_str().unwrap().to_string()];
    if let Some(s) = show {
        args.push("-s".into());
        args.push(s.into());
    }
    let out = repo39_bin()
        .args(&args)
        .output()
        .expect("failed to run repo39");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let lines = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    (tmp, lines)
}

#[test]
fn default_shows_files_and_dirs_no_hidden() {
    let (_tmp, l) = run_on_tree(None);

    assert!(l.iter().any(|s| s == "d src/"));
    assert!(l.iter().any(|s| s == "d src/nested/"));
    assert!(l.iter().any(|s| s.starts_with("f readme.txt ")));
    assert!(l.iter().any(|s| s.starts_with("f src/main.rs ")));
    assert!(l.iter().any(|s| s.starts_with("f src/nested/lib.rs ")));

    assert!(!l.iter().any(|s| s.contains(".secret")));
    assert!(!l.iter().any(|s| s.contains(".hidden_dir")));
}

#[test]
fn files_only() {
    let (_tmp, l) = run_on_tree(Some("f"));

    assert!(l.iter().any(|s| s.starts_with("f readme.txt ")));
    assert!(l.iter().any(|s| s.starts_with("f src/main.rs ")));
    assert!(!l.iter().any(|s| s.starts_with("d ")));
}

#[test]
fn dirs_only() {
    let (_tmp, l) = run_on_tree(Some("d"));

    assert!(l.iter().any(|s| s == "d src/"));
    assert!(l.iter().any(|s| s == "d src/nested/"));
    assert!(!l.iter().any(|s| s.starts_with("f ")));
}

#[test]
fn show_hidden() {
    let (_tmp, l) = run_on_tree(Some("fdh"));

    assert!(l.iter().any(|s| s.contains(".secret")));
    assert!(l.iter().any(|s| s.contains(".hidden_dir")));
}

#[test]
fn show_all() {
    let (_tmp, l) = run_on_tree(Some("a"));

    assert!(l.iter().any(|s| s.starts_with("f ")));
    assert!(l.iter().any(|s| s.starts_with("d ")));
    assert!(l.iter().any(|s| s.contains(".secret")));
    assert!(l.iter().any(|s| s.contains(".hidden_dir")));
}

#[test]
fn skips_noisy_dirs() {
    let (_tmp, l) = run_on_tree(Some("a"));

    assert!(!l.iter().any(|s| s.contains("node_modules")));
}

#[test]
fn sorted_output() {
    let (_tmp, l) = run_on_tree(Some("f"));

    let root_files: Vec<&str> = l.iter()
        .filter(|s| s.starts_with("f ") && !s.contains('/'))
        .map(|s| s.as_str())
        .collect();
    let mut sorted = root_files.clone();
    sorted.sort();
    assert_eq!(root_files, sorted);
}

#[test]
fn nonexistent_path_fails() {
    let out = repo39_bin()
        .args(["/tmp/repo39_does_not_exist_ever"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn filter_parse_a_enables_all() {
    let (_tmp_a, l_a) = run_on_tree(Some("a"));
    let (_tmp_fdh, l_fdh) = run_on_tree(Some("fdh"));

    assert_eq!(l_a, l_fdh);
}

#[test]
fn file_sizes_are_correct() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("five.txt"), "hello").unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "-s", "f"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("f five.txt 5"));
}
