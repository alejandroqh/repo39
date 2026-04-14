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

fn run_on_tree(show: Option<&str>, depth: Option<usize>) -> (tempfile::TempDir, Vec<String>) {
    run_on_tree_grep(show, depth, None)
}

fn run_on_tree_grep(show: Option<&str>, depth: Option<usize>, grep: Option<&str>) -> (tempfile::TempDir, Vec<String>) {
    run_on_tree_full(show, depth, grep, None, None, None)
}

fn run_on_tree_full(
    show: Option<&str>,
    depth: Option<usize>,
    grep: Option<&str>,
    order: Option<&str>,
    info: Option<&str>,
    unit: Option<&str>,
) -> (tempfile::TempDir, Vec<String>) {
    run_on_tree_all(show, depth, grep, order, info, unit, None)
}

fn run_on_tree_all(
    show: Option<&str>,
    depth: Option<usize>,
    grep: Option<&str>,
    order: Option<&str>,
    info: Option<&str>,
    unit: Option<&str>,
    limit: Option<usize>,
) -> (tempfile::TempDir, Vec<String>) {
    let tmp = tempfile::tempdir().unwrap();
    create_tree(tmp.path());
    let mut args = vec![tmp.path().to_str().unwrap().to_string()];
    if let Some(s) = show { args.push("-s".into()); args.push(s.into()); }
    if let Some(d) = depth { args.push("-d".into()); args.push(d.to_string()); }
    if let Some(g) = grep { args.push("-g".into()); args.push(g.into()); }
    if let Some(o) = order { args.push("-o".into()); args.push(o.into()); }
    if let Some(i) = info { args.push("-i".into()); args.push(i.into()); }
    if let Some(u) = unit { args.push("-u".into()); args.push(u.into()); }
    if let Some(n) = limit { args.push("-n".into()); args.push(n.to_string()); }
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

// --- show filter tests (use -d 99 for full depth) ---

#[test]
fn full_depth_shows_files_and_dirs_no_hidden() {
    let (_tmp, l) = run_on_tree(None, Some(99));

    assert!(l.contains(&"src/".into()));
    assert!(l.contains(&" nested/".into()));
    assert!(l.contains(&"readme.txt".into()));
    assert!(l.contains(&" main.rs".into()));
    assert!(l.contains(&"  lib.rs".into()));

    assert!(!l.iter().any(|s| s.contains(".secret")));
    assert!(!l.iter().any(|s| s.contains(".hidden_dir")));
}

#[test]
fn files_only() {
    let (_tmp, l) = run_on_tree(Some("f"), Some(99));

    assert!(l.contains(&"readme.txt".into()));
    assert!(l.contains(&" main.rs".into()));
    assert!(!l.iter().any(|s| s.trim_start().ends_with('/')));
}

#[test]
fn dirs_only() {
    let (_tmp, l) = run_on_tree(Some("d"), Some(99));

    assert!(l.contains(&"src/".into()));
    assert!(l.contains(&" nested/".into()));
    assert!(!l.iter().any(|s| !s.trim_start().ends_with('/')));
}

#[test]
fn show_hidden() {
    let (_tmp, l) = run_on_tree(Some("fdh"), Some(99));

    assert!(l.iter().any(|s| s.contains(".secret")));
    assert!(l.iter().any(|s| s.contains(".hidden_dir")));
}

#[test]
fn show_all() {
    let (_tmp, l) = run_on_tree(Some("a"), Some(99));

    assert!(l.iter().any(|s| !s.trim_start().ends_with('/')));
    assert!(l.iter().any(|s| s.trim_start().ends_with('/')));
    assert!(l.iter().any(|s| s.contains(".secret")));
    assert!(l.iter().any(|s| s.contains(".hidden_dir")));
}

#[test]
fn skips_noisy_dirs() {
    let (_tmp, l) = run_on_tree(Some("a"), Some(99));

    assert!(!l.iter().any(|s| s.contains("node_modules")));
}

#[test]
fn sorted_output() {
    let (_tmp, l) = run_on_tree(Some("f"), Some(99));

    let root_files: Vec<&str> = l.iter()
        .filter(|s| !s.starts_with(' '))
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
    let (_tmp_a, l_a) = run_on_tree(Some("a"), Some(99));
    let (_tmp_fdh, l_fdh) = run_on_tree(Some("fdh"), Some(99));

    assert_eq!(l_a, l_fdh);
}

#[test]
fn indentation_reflects_depth() {
    let (_tmp, l) = run_on_tree(None, Some(99));

    assert!(l.contains(&"src/".into()));
    assert!(l.contains(&" nested/".into()));
    assert!(l.contains(&"  lib.rs".into()));
}

// --- depth limit tests ---

#[test]
fn depth_zero_root_only() {
    let (_tmp, l) = run_on_tree(None, None);

    // no indented lines — root only
    assert!(!l.iter().any(|s| s.starts_with(' ')));
    // has root-level entries
    assert!(l.contains(&"readme.txt".into()));
    assert!(l.contains(&"src/".into()));
}

#[test]
fn depth_one_level() {
    let (_tmp, l) = run_on_tree(None, Some(1));

    // depth 0 entries
    assert!(l.contains(&"src/".into()));
    assert!(l.contains(&"readme.txt".into()));
    // depth 1 entries (1 space indent)
    assert!(l.contains(&" main.rs".into()));
    assert!(l.contains(&" nested/".into()));
    // no depth 2 entries (2 space indent)
    assert!(!l.iter().any(|s| s.starts_with("  ")));
}

// --- count tests ---

#[test]
fn count_on_truncated_dirs() {
    let (_tmp, l) = run_on_tree(Some("fdc"), None);

    // src has 2 files: main.rs + nested/lib.rs
    assert!(l.contains(&"src/ 2".into()));
}

#[test]
fn count_without_depth_limit_noop() {
    let (_tmp, l) = run_on_tree(Some("fdc"), Some(99));

    // all dirs expanded, no counts
    assert!(l.contains(&"src/".into()));
    assert!(!l.iter().any(|s| s.starts_with("src/ ")));
}

#[test]
fn count_respects_hidden() {
    let (_tmp, l) = run_on_tree(Some("fdch"), None);

    // .hidden_dir has 1 file: data
    assert!(l.iter().any(|s| s.starts_with(".hidden_dir/ 1")));
}

// --- grep tests ---

#[test]
fn grep_exact_filename() {
    let (_tmp, l) = run_on_tree_grep(None, None, Some("readme.txt"));

    assert!(l.contains(&"readme.txt".into()));
    // no other files
    assert_eq!(l.len(), 1);
}

#[test]
fn grep_star_extension() {
    let (_tmp, l) = run_on_tree_grep(None, None, Some("*.rs"));

    // main.rs and lib.rs with their parent dirs
    assert!(l.contains(&"src/".into()));
    assert!(l.contains(&" main.rs".into()));
    assert!(l.contains(&" nested/".into()));
    assert!(l.contains(&"  lib.rs".into()));
    // no non-rs files
    assert!(!l.iter().any(|s| s.trim_start() == "readme.txt"));
}

#[test]
fn grep_prefix_star() {
    let (_tmp, l) = run_on_tree_grep(None, None, Some("main*"));

    assert!(l.contains(&"src/".into()));
    assert!(l.contains(&" main.rs".into()));
}

#[test]
fn grep_no_match_empty_output() {
    let (_tmp, l) = run_on_tree_grep(None, None, Some("nonexistent.xyz"));

    assert!(l.is_empty());
}

#[test]
fn grep_shows_ancestor_dirs() {
    let (_tmp, l) = run_on_tree_grep(None, None, Some("lib.rs"));

    // lib.rs is in src/nested/ — both ancestors must appear
    assert!(l.contains(&"src/".into()));
    assert!(l.contains(&" nested/".into()));
    assert!(l.contains(&"  lib.rs".into()));
    assert_eq!(l.len(), 3);
}

#[test]
fn grep_respects_hidden_filter() {
    // without hidden flag, .secret should not match
    let (_tmp, l) = run_on_tree_grep(None, None, Some(".secret"));
    assert!(l.is_empty());

    // with hidden flag, .secret matches
    let (_tmp, l) = run_on_tree_grep(Some("fdh"), None, Some(".secret"));
    assert!(l.contains(&".secret".into()));
}

#[test]
fn grep_star_matches_all() {
    let (_tmp, l) = run_on_tree_grep(None, None, Some("*"));

    assert!(l.iter().any(|s| s.trim_start() == "readme.txt"));
    assert!(l.iter().any(|s| s.trim_start() == "main.rs"));
    assert!(l.iter().any(|s| s.trim_start() == "lib.rs"));
}

// --- info/sort/unit tests ---

#[test]
fn info_size_shows_size_suffix() {
    let (_tmp, l) = run_on_tree_full(None, Some(99), None, None, Some("s"), None);

    // readme.txt is 5 bytes → "0.0K"
    assert!(l.iter().any(|s| s.contains("readme.txt") && s.contains("K")));
}

#[test]
fn info_size_mb_unit() {
    let (_tmp, l) = run_on_tree_full(None, Some(99), None, None, Some("s"), Some("M"));

    assert!(l.iter().any(|s| s.contains("readme.txt") && s.contains("M")));
}

#[test]
fn sort_by_size_largest_first() {
    let (_tmp, l) = run_on_tree_full(Some("f"), Some(99), None, Some("s"), None, None);

    // "fn main() {}" (12 bytes) > "hello" (5 bytes) > "// lib" (6 bytes)
    // main.rs should appear before readme.txt at their respective depths
    let root_files: Vec<&str> = l.iter()
        .filter(|s| !s.starts_with(' '))
        .map(|s| s.as_str())
        .collect();
    // readme.txt (5 bytes) is the only root file, so just check it has size
    assert!(root_files.iter().any(|s| s.contains("readme.txt") && s.contains("K")));
}

#[test]
fn sort_by_modified_shows_date() {
    let (_tmp, l) = run_on_tree_full(None, Some(99), None, Some("m"), None, None);

    // all files should have a date like YYYY-MM-DD
    assert!(l.iter().filter(|s| !s.trim_start().ends_with('/')).all(|s| {
        s.contains("-") && s.len() > 10
    }));
}

#[test]
fn info_modified_shows_date() {
    let (_tmp, l) = run_on_tree_full(None, Some(99), None, None, Some("m"), None);

    assert!(l.iter().any(|s| s.contains("readme.txt") && s.contains("20")));
}

#[test]
fn info_git_on_non_git_warns() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.txt"), "hi").unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "-i", "g"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not a git repo"));
}

#[test]
fn info_multiple_fields() {
    let (_tmp, l) = run_on_tree_full(None, Some(99), None, None, Some("sm"), None);

    // should have both size (K) and date (YYYY-)
    assert!(l.iter().any(|s| s.contains("K") && s.contains("20")));
}

// --- limit tests ---

#[test]
fn limit_files_per_dir() {
    // tree has 1 root file (readme.txt) + 2 dirs (src/, .hidden_dir via node_modules skipped)
    // src/ has 1 file (main.rs) + 1 dir (nested/)
    // nested/ has 1 file (lib.rs)
    let (_tmp, l) = run_on_tree_all(Some("f"), Some(99), None, None, None, None, Some(1));

    // root: readme.txt shown, no "+more" (only 1 file)
    assert!(l.contains(&"readme.txt".into()));

    // src/ depth 1: main.rs shown, no "+more" (only 1 file)
    assert!(l.contains(&" main.rs".into()));
}

#[test]
fn limit_shows_ellipsis_in_subdir() {
    // limit only applies at depth > 0, so put files in a subdir
    let tmp = tempfile::tempdir().unwrap();
    let sub = tmp.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("a.txt"), "a").unwrap();
    fs::write(sub.join("b.txt"), "b").unwrap();
    fs::write(sub.join("c.txt"), "c").unwrap();
    fs::write(sub.join("d.txt"), "d").unwrap();
    fs::write(sub.join("e.txt"), "e").unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "-d", "1", "-n", "2"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    // sub/ + 2 files + "...+3"
    assert!(stdout.contains("sub/"));
    assert!(stdout.contains("...+3"));
}

#[test]
fn limit_does_not_apply_to_root() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.txt"), "a").unwrap();
    fs::write(tmp.path().join("b.txt"), "b").unwrap();
    fs::write(tmp.path().join("c.txt"), "c").unwrap();
    fs::write(tmp.path().join("d.txt"), "d").unwrap();
    fs::write(tmp.path().join("e.txt"), "e").unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "-n", "2"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    // root is unlimited — all 5 files shown
    assert_eq!(stdout.lines().count(), 5);
    assert!(!stdout.contains("..."));
}

#[test]
fn limit_truncates_dirs_in_subdir() {
    let tmp = tempfile::tempdir().unwrap();
    let sub = tmp.path().join("parent");
    fs::create_dir_all(sub.join("aaa")).unwrap();
    fs::create_dir_all(sub.join("bbb")).unwrap();
    fs::create_dir_all(sub.join("ccc")).unwrap();
    fs::write(sub.join("x.txt"), "x").unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "-d", "2", "-n", "1"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    // inside parent/: 1 file + 1 dir shown, 2 dirs hidden = ...+2
    assert!(stdout.contains("...+2"));
}

#[test]
fn limit_zero_is_unlimited() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.txt"), "a").unwrap();
    fs::write(tmp.path().join("b.txt"), "b").unwrap();
    fs::write(tmp.path().join("c.txt"), "c").unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "-n", "0"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert_eq!(stdout.lines().count(), 3);
    assert!(!stdout.contains("..."));
}

#[test]
fn limit_does_not_affect_dirs() {
    let (_tmp, l) = run_on_tree_all(None, Some(99), None, None, None, None, Some(1));

    // dirs still shown even with limit=1
    assert!(l.iter().any(|s| s.trim_start().ends_with('/')));
}
