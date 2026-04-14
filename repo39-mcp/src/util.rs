use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    ".next",
    "build",
    ".build",
    ".dart_tool",
    ".gradle",
    "Pods",
    "Flutter",
    "DerivedData",
    ".idea",
    ".vs",
    "bin",
    "obj",
    "out",
    "vendor",
    ".cache",
];

const INDENT_BUF: &[u8; 64] = b"                                                                ";

pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

pub fn should_skip(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

pub fn write_indent(out: &mut impl Write, depth: usize) -> io::Result<()> {
    let n = depth.min(INDENT_BUF.len());
    out.write_all(&INDENT_BUF[..n])
}

pub fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    let canonical = std::fs::canonicalize(path)?;

    #[cfg(target_os = "windows")]
    {
        let s = canonical.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(stripped));
        }
    }

    Ok(canonical)
}

pub fn rel_path(dir: &Path, root: &Path, name: &str) -> String {
    let full = dir.join(name);
    match full.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().into_owned(),
        Err(_) => name.to_string(),
    }
}

pub fn systime_to_epoch(t: Option<SystemTime>) -> u64 {
    t.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn epoch_to_date(epoch: u64) -> String {
    if epoch == 0 {
        return "-".to_string();
    }
    const DAYS_PER_YEAR: u64 = 365;
    const SECS_PER_DAY: u64 = 86400;

    let days = epoch / SECS_PER_DAY;

    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / DAYS_PER_YEAR;
    let y = yoe + era * 400;
    let doy = doe - (DAYS_PER_YEAR * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}")
}
