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

const INDENT_BUF: &[u8; 64] = b"                                                                ";

#[derive(Parser)]
#[command(
    name = "repo39",
    version,
    about = "Token-optimized repo explorer for AI agents"
)]
struct Cli {
    /// Target directory (relative or absolute)
    path: PathBuf,

    /// Show filter: f=files d=dirs h=hidden c=count a=all [default: fd]
    #[arg(short, long, default_value = "fd")]
    show: String,

    /// Max depth (0=root only, default)
    #[arg(short, long, default_value = "0")]
    depth: usize,
}

struct ShowFilter {
    files: bool,
    dirs: bool,
    hidden: bool,
    count: bool,
    max_depth: usize,
}

impl ShowFilter {
    fn parse(s: &str, max_depth: usize) -> Self {
        let count = s.contains('c');
        if s.contains('a') {
            return Self { files: true, dirs: true, hidden: true, count, max_depth };
        }
        Self {
            files: s.contains('f'),
            dirs: s.contains('d'),
            hidden: s.contains('h'),
            count,
            max_depth,
        }
    }
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let root = canonicalize(&cli.path)?;
    let filter = ShowFilter::parse(&cli.show, cli.depth);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    walk(&root, &filter, 0, &mut out)
}

fn walk(dir: &Path, filter: &ShowFilter, depth: usize, out: &mut impl Write) -> io::Result<()> {
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
            let at_limit = depth >= filter.max_depth;
            if filter.dirs {
                write_indent(out, depth)?;
                out.write_all(name_str.as_bytes())?;
                if at_limit && filter.count {
                    let n = count_files(&entry.path(), filter)?;
                    writeln!(out, "/ {n}")?;
                } else {
                    writeln!(out, "/")?;
                }
            }
            if !at_limit {
                walk(&entry.path(), filter, depth + 1, out)?;
            }
        } else if ft.is_file() && filter.files {
            write_indent(out, depth)?;
            out.write_all(name_str.as_bytes())?;
            writeln!(out)?;
        }
    }

    Ok(())
}

fn count_files(dir: &Path, filter: &ShowFilter) -> io::Result<usize> {
    let mut total = 0;
    let entries = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(0),
    };

    for entry in entries.filter_map(|e| e.ok()) {
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
            if !should_skip(name_str) {
                total += count_files(&entry.path(), filter)?;
            }
        } else if ft.is_file() {
            total += 1;
        }
    }

    Ok(total)
}

fn write_indent(out: &mut impl Write, depth: usize) -> io::Result<()> {
    let n = depth.min(INDENT_BUF.len());
    out.write_all(&INDENT_BUF[..n])
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
