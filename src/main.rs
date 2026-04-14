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

    /// Grep files by name glob (e.g. "*.json", "pack*", "Cargo.toml")
    #[arg(short, long)]
    grep: Option<String>,
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

/// Pre-compiled glob pattern to avoid per-file allocations.
enum Glob {
    All,
    Exact(String),
    Parts { segments: Vec<String>, anchored_start: bool, anchored_end: bool },
}

impl Glob {
    fn compile(pattern: &str) -> Self {
        if pattern == "*" {
            return Self::All;
        }
        if !pattern.contains('*') {
            return Self::Exact(pattern.to_string());
        }
        let raw_parts: Vec<&str> = pattern.split('*').collect();
        let anchored_start = !raw_parts[0].is_empty();
        let anchored_end = !raw_parts[raw_parts.len() - 1].is_empty();
        let segments: Vec<String> = raw_parts.iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Self::Parts { segments, anchored_start, anchored_end }
    }

    fn matches(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Exact(p) => p == name,
            Self::Parts { segments, anchored_start, anchored_end } => {
                if segments.is_empty() {
                    return true;
                }
                let mut pos = 0;
                for (i, seg) in segments.iter().enumerate() {
                    if i == 0 && *anchored_start {
                        if !name.starts_with(seg.as_str()) {
                            return false;
                        }
                        pos = seg.len();
                    } else if i == segments.len() - 1 && *anchored_end {
                        if !name[pos..].ends_with(seg.as_str()) {
                            return false;
                        }
                        return true;
                    } else {
                        match name[pos..].find(seg.as_str()) {
                            Some(idx) => pos += idx + seg.len(),
                            None => return false,
                        }
                    }
                }
                true
            }
        }
    }
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut dir_buf = canonicalize(&cli.path)?;
    let filter = ShowFilter::parse(&cli.show, cli.depth);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    if let Some(ref pattern) = cli.grep {
        let glob = Glob::compile(pattern);
        grep_walk(&mut dir_buf, &filter, &glob, 0, &mut out)?;
    } else {
        walk(&mut dir_buf, &filter, 0, &mut out)?;
    }

    Ok(())
}

fn walk(dir_buf: &mut PathBuf, filter: &ShowFilter, depth: usize, out: &mut impl Write) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir_buf.as_path())?
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
                    dir_buf.push(name_str);
                    let n = count_files(dir_buf, filter)?;
                    dir_buf.pop();
                    writeln!(out, "/ {n}")?;
                } else {
                    writeln!(out, "/")?;
                }
            }
            if !at_limit {
                dir_buf.push(name_str);
                walk(dir_buf, filter, depth + 1, out)?;
                dir_buf.pop();
            }
        } else if ft.is_file() && filter.files {
            write_indent(out, depth)?;
            out.write_all(name_str.as_bytes())?;
            writeln!(out)?;
        }
    }

    Ok(())
}

/// Walk full depth, show only files matching glob + their ancestor dirs.
/// Returns true if any match was found in this subtree.
fn grep_walk(
    dir_buf: &mut PathBuf,
    filter: &ShowFilter,
    glob: &Glob,
    depth: usize,
    out: &mut impl Write,
) -> io::Result<bool> {
    let mut entries: Vec<_> = fs::read_dir(dir_buf.as_path())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut found = false;

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
            let mut buf = Vec::new();
            dir_buf.push(name_str);
            let has_matches = grep_walk(dir_buf, filter, glob, depth + 1, &mut buf)?;
            dir_buf.pop();
            if has_matches {
                found = true;
                write_indent(out, depth)?;
                out.write_all(name_str.as_bytes())?;
                writeln!(out, "/")?;
                out.write_all(&buf)?;
            }
        } else if ft.is_file() && glob.matches(name_str) {
            found = true;
            write_indent(out, depth)?;
            out.write_all(name_str.as_bytes())?;
            writeln!(out)?;
        }
    }

    Ok(found)
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
