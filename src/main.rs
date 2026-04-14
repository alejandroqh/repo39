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
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let root = fs::canonicalize(&cli.path)?;

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    walk(&root, &root, &mut out)
}

fn walk(root: &Path, dir: &Path, out: &mut impl Write) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let meta = entry.metadata()?;

        if meta.is_dir() {
            if should_skip(&entry) {
                continue;
            }
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            writeln!(out, "d {}/", rel.display())?;
            walk(root, &path, out)?;
        } else if meta.is_file() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            writeln!(out, "f {} {}", rel.display(), meta.len())?;
        }
    }

    Ok(())
}

fn should_skip(entry: &fs::DirEntry) -> bool {
    let name = entry.file_name();
    let name = name.to_str().unwrap_or("");
    SKIP_DIRS.contains(&name)
}
