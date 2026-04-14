use std::{
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use clap::Parser;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    ".next",
];

#[derive(Parser)]
#[command(
    name = "repo39",
    version,
    about = "Token-optimized repo explorer for AI agents"
)]
struct Cli {
    /// Target directory (relative or absolute)
    path: PathBuf,

    /// Show filter: f=files d=dirs h=hidden a=all [default: fd]
    #[arg(short, long, default_value = "fd")]
    show: String,
}

struct ShowFilter {
    files: bool,
    dirs: bool,
    hidden: bool,
}

impl ShowFilter {
    fn parse(s: &str) -> Self {
        if s.contains('a') {
            return Self { files: true, dirs: true, hidden: true };
        }
        Self {
            files: s.contains('f'),
            dirs: s.contains('d'),
            hidden: s.contains('h'),
        }
    }
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let root = canonicalize(&cli.path)?;
    let filter = ShowFilter::parse(&cli.show);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    walk(&root, &root, &filter, &mut out)
}

fn walk(root: &Path, dir: &Path, filter: &ShowFilter, out: &mut impl Write) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_str().unwrap_or("");

        if !filter.hidden && is_hidden(name_str) {
            continue;
        }

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if ft.is_dir() {
            if should_skip(name_str) {
                continue;
            }
            let path = entry.path();
            if filter.dirs {
                write_rel(out, b"d ", root, &path, None)?;
            }
            walk(root, &path, filter, out)?;
        } else if ft.is_file() && filter.files {
            let path = entry.path();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            write_rel(out, b"f ", root, &path, Some(size))?;
        }
    }

    Ok(())
}

fn write_rel(out: &mut impl Write, prefix: &[u8], root: &Path, path: &Path, size: Option<u64>) -> io::Result<()> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    out.write_all(prefix)?;

    #[cfg(target_os = "windows")]
    {
        let s = rel.to_string_lossy();
        out.write_all(s.replace('\\', "/").as_bytes())?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::ffi::OsStrExt;
        out.write_all(rel.as_os_str().as_bytes())?;
    }

    if let Some(size) = size {
        write!(out, " {size}")?;
    } else {
        out.write_all(b"/")?;
    }
    writeln!(out)
}

fn should_skip(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Canonicalize without Windows UNC prefix (\\?\).
fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;

    #[cfg(target_os = "windows")]
    {
        let s = canonical.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(stripped));
        }
    }

    Ok(canonical)
}
