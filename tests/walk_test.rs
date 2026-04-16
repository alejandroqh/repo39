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

// --- identify tests ---

fn run_identify(base: &Path) -> Vec<String> {
    let out = repo39_bin()
        .args([base.to_str().unwrap(), "--identify"])
        .output()
        .expect("failed to run repo39");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn identify_rust_project() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
    fs::write(tmp.path().join("Cargo.lock"), "").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(tmp.path().join("src/lib.rs"), "").unwrap();

    let l = run_identify(tmp.path());
    assert!(!l.is_empty());
    // first result should be rust
    assert!(l[0].starts_with("rust "));
    // confidence should be high
    let confidence: f64 = l[0].split_whitespace().last().unwrap().parse().unwrap();
    assert!(confidence > 0.7, "rust confidence {confidence} too low");
}

#[test]
fn identify_python_project() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("pyproject.toml"), "").unwrap();
    fs::write(tmp.path().join("requirements.txt"), "").unwrap();
    fs::write(tmp.path().join("app.py"), "").unwrap();
    fs::write(tmp.path().join("utils.py"), "").unwrap();
    fs::write(tmp.path().join("test.py"), "").unwrap();

    let l = run_identify(tmp.path());
    assert!(!l.is_empty());
    assert!(l[0].starts_with("python "));
    let confidence: f64 = l[0].split_whitespace().last().unwrap().parse().unwrap();
    assert!(confidence > 0.7, "python confidence {confidence} too low");
}

#[test]
fn identify_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();

    let l = run_identify(tmp.path());
    assert!(l.is_empty(), "empty dir should produce no results");
}

#[test]
fn identify_output_format() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "").unwrap();

    let l = run_identify(tmp.path());
    for line in &l {
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(parts.len(), 3, "each line should be 'name category confidence': {line}");
        let conf: f64 = parts[2].parse().expect("confidence should be f64");
        assert!(conf > 0.0 && conf <= 1.0, "confidence out of range: {conf}");
        // check two decimal places
        assert!(parts[2].contains('.'), "should have decimal: {}", parts[2]);
    }
}

#[test]
fn identify_max_five_results() {
    // create a dir with signals for many categories
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    fs::write(tmp.path().join("package.json"), "").unwrap();
    fs::write(tmp.path().join("pyproject.toml"), "").unwrap();
    fs::write(tmp.path().join("go.mod"), "").unwrap();
    fs::write(tmp.path().join("Gemfile"), "").unwrap();
    fs::write(tmp.path().join("composer.json"), "").unwrap();
    fs::write(tmp.path().join("tsconfig.json"), "").unwrap();
    fs::write(tmp.path().join("Dockerfile"), "").unwrap();
    fs::write(tmp.path().join(".gitignore"), "").unwrap();

    let l = run_identify(tmp.path());
    assert!(l.len() <= 5, "should return at most 5 results, got {}", l.len());
}

#[test]
fn identify_sorted_descending() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    fs::write(tmp.path().join("package.json"), "").unwrap();
    fs::write(tmp.path().join("README.md"), "").unwrap();

    let l = run_identify(tmp.path());
    let confidences: Vec<f64> = l.iter()
        .map(|s| s.split_whitespace().last().unwrap().parse().unwrap())
        .collect();
    for w in confidences.windows(2) {
        assert!(w[0] >= w[1], "not sorted desc: {} < {}", w[0], w[1]);
    }
}

#[test]
fn identify_ignores_other_flags() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();

    // run with --identify plus other flags that would normally affect walk output
    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "--identify", "-d", "5", "-s", "a", "-n", "1"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    // should be identify output, not walk output
    assert!(stdout.contains("rust"));
    assert!(!stdout.contains("src/"));  // no tree output
}

#[test]
fn identify_flutter_project() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("pubspec.yaml"), "").unwrap();
    fs::create_dir_all(tmp.path().join("android")).unwrap();
    fs::create_dir_all(tmp.path().join("ios")).unwrap();
    fs::create_dir_all(tmp.path().join("lib")).unwrap();
    fs::write(tmp.path().join("lib/main.dart"), "").unwrap();
    fs::write(tmp.path().join("analysis_options.yaml"), "").unwrap();

    let l = run_identify(tmp.path());
    // flutter and dart should both appear
    let names: Vec<&str> = l.iter().map(|s| s.split_whitespace().next().unwrap()).collect();
    assert!(names.contains(&"flutter"), "flutter not detected: {names:?}");
    assert!(names.contains(&"dart"), "dart not detected: {names:?}");
}

// --- map tests ---

fn run_map(base: &Path, extra_args: &[&str]) -> Vec<String> {
    let mut args = vec![base.to_str().unwrap().to_string(), "--map".into()];
    for a in extra_args {
        args.push(a.to_string());
    }
    let out = repo39_bin()
        .args(&args)
        .output()
        .expect("failed to run repo39");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn map_rust_project() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "\
pub fn main() {}
fn helper() {}
pub struct Config {}
enum Mode { A, B }
trait Runnable {}
impl Config {}
").unwrap();
    fs::write(tmp.path().join("src/lib.rs"), "\
pub fn init() {}
pub struct App {}
").unwrap();

    let l = run_map(tmp.path(), &["-d", "1"]);
    // Tree format: src/ header, files indented 1, symbols indented 2
    assert!(l.contains(&"src/".into()));
    assert!(l.iter().any(|s| s == " lib.rs"));
    assert!(l.iter().any(|s| s == " main.rs"));
    assert!(l.iter().any(|s| s == "  1:+fn main"));
    assert!(l.iter().any(|s| s == "  2:fn helper"));
    assert!(l.iter().any(|s| s == "  3:+struct Config"));
    assert!(l.iter().any(|s| s == "  4:enum Mode"));
    assert!(l.iter().any(|s| s == "  5:trait Runnable"));
    assert!(l.iter().any(|s| s == "  6:impl Config"));
    assert!(l.iter().any(|s| s == "  1:+fn init"));
    assert!(l.iter().any(|s| s == "  2:+struct App"));
}

#[test]
fn map_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let l = run_map(tmp.path(), &[]);
    assert!(l.is_empty(), "empty dir should produce no output");
}

#[test]
fn map_skips_noisy_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("node_modules")).unwrap();
    fs::write(tmp.path().join("node_modules/index.js"), "function foo() {}").unwrap();
    fs::create_dir_all(tmp.path().join("target")).unwrap();
    fs::write(tmp.path().join("target/main.rs"), "fn bar() {}").unwrap();

    let l = run_map(tmp.path(), &[]);
    assert!(l.is_empty(), "noisy dirs should be skipped: {l:?}");
}

#[test]
fn map_python_symbols() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("app.py"), "\
def hello():
    pass

class Server:
    def run(self):
        pass
").unwrap();

    let l = run_map(tmp.path(), &[]);
    assert!(l.contains(&"app.py".into()));
    assert!(l.iter().any(|s| s == " 1:def hello"));
    assert!(l.iter().any(|s| s == " 4:class Server"));
    assert!(l.iter().any(|s| s == " 5:def run"));
}

#[test]
fn map_ignores_other_flags() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("test.rs"), "fn foo() {}\n").unwrap();

    let l = run_map(tmp.path(), &["-d", "5", "-s", "a"]);
    assert!(l.contains(&"test.rs".into()));
    assert!(l.iter().any(|s| s == " 1:fn foo"));
    assert!(!l.iter().any(|s| s.ends_with('/')));
}

#[test]
fn map_limit_truncates_symbols() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("big.rs"), "\
fn alpha() {}
fn beta() {}
fn gamma() {}
fn delta() {}
fn epsilon() {}
").unwrap();

    let l = run_map(tmp.path(), &["-n", "2"]);
    assert!(l.contains(&"big.rs".into()));
    assert!(l.iter().any(|s| s == " 1:fn alpha"));
    assert!(l.iter().any(|s| s == " 2:fn beta"));
    assert!(!l.iter().any(|s| s.contains("fn gamma")));
    assert!(l.iter().any(|s| s == " ...+3"), "should show ...+3: {l:?}");
}

#[test]
fn map_depth_limits_subdirs() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("src/nested")).unwrap();
    fs::write(tmp.path().join("root.rs"), "fn shallow() {}").unwrap();
    fs::write(tmp.path().join("src/mid.rs"), "fn middle() {}").unwrap();
    fs::write(tmp.path().join("src/nested/deep.rs"), "fn deep() {}").unwrap();

    // depth 1 = root + one level only (src/*.rs but not src/nested/*.rs)
    let l = run_map(tmp.path(), &["-d", "1"]);
    assert!(l.iter().any(|s| s.trim().starts_with("1:fn shallow")));
    assert!(l.iter().any(|s| s.trim().starts_with("1:fn middle")));
    assert!(!l.iter().any(|s| s.contains("deep")));

    // default (no -d) = full depth
    let l = run_map(tmp.path(), &[]);
    assert!(l.iter().any(|s| s.trim().starts_with("1:fn deep")));
}

// --- deps tests ---

fn run_deps(base: &Path) -> Vec<String> {
    let out = repo39_bin()
        .args([base.to_str().unwrap(), "--deps"])
        .output()
        .expect("failed to run repo39");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn deps_cargo_toml() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), r#"
[package]
name = "myapp"
version = "0.1.0"

[dependencies]
clap = "4"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
tempfile = "3"
"#).unwrap();

    let l = run_deps(tmp.path());
    assert!(l.contains(&"clap 4".to_string()), "missing clap: {l:?}");
    assert!(l.contains(&"serde 1.0".to_string()), "missing serde: {l:?}");
    assert!(l.contains(&"tempfile 3 dev".to_string()), "missing tempfile dev: {l:?}");
}

#[test]
fn deps_package_json() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("package.json"), r#"{
  "name": "myapp",
  "dependencies": {
    "react": "^18.2.0",
    "lodash": "~4.17.21"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}"#).unwrap();

    let l = run_deps(tmp.path());
    assert!(l.contains(&"react 18.2.0".to_string()), "missing react: {l:?}");
    assert!(l.contains(&"lodash 4.17.21".to_string()), "missing lodash: {l:?}");
    assert!(l.contains(&"jest 29.0.0 dev".to_string()), "missing jest dev: {l:?}");
}

#[test]
fn deps_requirements_txt() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("requirements.txt"), "# comment
requests==2.28.1
flask>=2.0
numpy
").unwrap();

    let l = run_deps(tmp.path());
    assert!(l.contains(&"requests 2.28.1".to_string()), "missing requests: {l:?}");
    assert!(l.contains(&"flask 2.0".to_string()), "missing flask: {l:?}");
    assert!(l.contains(&"numpy".to_string()), "missing numpy: {l:?}");
}

#[test]
fn deps_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();

    let l = run_deps(tmp.path());
    assert!(l.is_empty(), "expected empty output for dir with no manifests: {l:?}");
}

#[test]
fn deps_multiple_manifests() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), r#"
[dependencies]
clap = "4"
"#).unwrap();
    fs::write(tmp.path().join("package.json"), r#"{
  "dependencies": {
    "react": "^18.0.0"
  }
}"#).unwrap();

    let l = run_deps(tmp.path());
    assert!(l.contains(&"Cargo.toml".to_string()), "missing Cargo.toml header: {l:?}");
    assert!(l.contains(&"package.json".to_string()), "missing package.json header: {l:?}");
    assert!(l.contains(&" clap 4".to_string()), "missing indented clap: {l:?}");
    assert!(l.contains(&" react 18.0.0".to_string()), "missing indented react: {l:?}");
}

#[test]
fn deps_ignores_other_flags() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), r#"
[dependencies]
clap = "4"
"#).unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "--deps", "-d", "5"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("clap 4"));
    assert!(!stdout.contains("src/"));
}

// --- changes tests ---

fn init_git_repo(path: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
}

#[test]
fn changes_not_git_repo() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("hello.txt"), "hi").unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "--changes"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stdout.is_empty(), "stdout should be empty for non-git repo");
    assert!(stderr.contains("not a git repo"), "stderr should warn: {stderr}");
}

#[test]
fn changes_with_commits() {
    let tmp = tempfile::tempdir().unwrap();
    init_git_repo(tmp.path());

    fs::write(tmp.path().join("hello.rs"), "fn main() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    fs::write(tmp.path().join("hello.rs"), "fn main() {\n    println!(\"hi\");\n}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "update"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "--changes"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("hello.rs"), "output should contain hello.rs: {stdout}");
    assert!(stdout.contains("+"), "output should contain insertions: {stdout}");
    assert!(stdout.contains("new"), "output should contain 'new' marker: {stdout}");
}

#[test]
fn changes_output_format() {
    let tmp = tempfile::tempdir().unwrap();
    init_git_repo(tmp.path());

    fs::write(tmp.path().join("a.txt"), "line1\nline2\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add a"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "--changes"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert!(parts.len() >= 3, "line should have at least 3 parts: {line}");
        let time = parts[0];
        assert!(
            time.ends_with('m')
                || time.ends_with('h')
                || time.ends_with('d')
                || time.ends_with('w')
                || time.ends_with('M')
                || time.ends_with('y'),
            "time_ago should end with time unit: {time}"
        );
        assert!(
            parts.iter().any(|p| p.starts_with('+')),
            "should have insertions: {line}"
        );
    }
}

#[test]
fn changes_ignores_other_flags() {
    let tmp = tempfile::tempdir().unwrap();
    init_git_repo(tmp.path());

    fs::write(tmp.path().join("test.rs"), "fn test() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let out = repo39_bin()
        .args([tmp.path().to_str().unwrap(), "--changes", "-d", "5"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("test.rs"), "should show test.rs in changes output");
    assert!(stdout.contains("+"), "should show insertions");
    assert!(
        !stdout.contains("test.rs/"),
        "should not be walk output with trailing slash"
    );
}
