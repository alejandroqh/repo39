use std::{
    collections::HashSet,
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
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

    /// Sort: n=name(default) s=size m=modified c=created
    #[arg(short, long, default_value = "n")]
    order: String,

    /// Info to display: s=size m=modified c=created g=git (combinable)
    #[arg(short, long, default_value = "")]
    info: String,

    /// Size unit: K=KB M=MB G=GB (default: K)
    #[arg(short, long, default_value = "K")]
    unit: String,
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

#[derive(Clone, Copy)]
enum SortOrder {
    Name,
    Size,
    Modified,
    Created,
}

impl SortOrder {
    fn parse(s: &str) -> Self {
        match s.chars().next().unwrap_or('n') {
            's' => Self::Size,
            'm' => Self::Modified,
            'c' => Self::Created,
            _ => Self::Name,
        }
    }
}

struct InfoFlags {
    size: bool,
    modified: bool,
    created: bool,
    git: bool,
}

impl InfoFlags {
    fn parse(s: &str, order: SortOrder) -> Self {
        let mut flags = Self {
            size: s.contains('s'),
            modified: s.contains('m'),
            created: s.contains('c'),
            git: s.contains('g'),
        };
        match order {
            SortOrder::Size => flags.size = true,
            SortOrder::Modified => flags.modified = true,
            SortOrder::Created => flags.created = true,
            SortOrder::Name => {}
        }
        flags
    }

    fn needs_metadata(&self) -> bool {
        self.size || self.modified || self.created
    }
}

#[derive(Clone, Copy)]
enum SizeUnit {
    K,
    M,
    G,
}

impl SizeUnit {
    fn parse(s: &str) -> Self {
        match s.chars().next().unwrap_or('K') {
            'M' | 'm' => Self::M,
            'G' | 'g' => Self::G,
            _ => Self::K,
        }
    }

    fn format(self, bytes: u64) -> String {
        match self {
            Self::K => {
                let kb = bytes as f64 / 1024.0;
                if kb < 10.0 { format!("{kb:.1}K") }
                else { format!("{}K", kb as u64) }
            }
            Self::M => {
                let mb = bytes as f64 / (1024.0 * 1024.0);
                if mb < 10.0 { format!("{mb:.2}M") }
                else { format!("{mb:.1}M") }
            }
            Self::G => {
                let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                format!("{gb:.2}G")
            }
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

struct WalkCtx<'a> {
    root: &'a Path,
    filter: ShowFilter,
    order: SortOrder,
    info: InfoFlags,
    unit: SizeUnit,
    dirty_files: HashSet<String>,
}

struct EntryInfo {
    name: String,
    is_dir: bool,
    size: u64,
    modified: u64,
    created: u64,
    dirty: bool,
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut dir_buf = canonicalize(&cli.path)?;
    let filter = ShowFilter::parse(&cli.show, cli.depth);
    let order = SortOrder::parse(&cli.order);
    let info = InfoFlags::parse(&cli.info, order);

    let dirty_files = if info.git {
        load_git_dirty(&dir_buf, true)
    } else {
        HashSet::new()
    };

    let ctx = WalkCtx {
        root: &dir_buf.clone(),
        filter,
        order,
        info,
        unit: SizeUnit::parse(&cli.unit),
        dirty_files,
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    if let Some(ref pattern) = cli.grep {
        let glob = Glob::compile(pattern);
        grep_walk(&mut dir_buf, &ctx, &glob, 0, &mut out)?;
    } else {
        walk(&mut dir_buf, &ctx, 0, &mut out)?;
    }

    Ok(())
}

fn collect_entries(dir_buf: &Path, ctx: &WalkCtx) -> io::Result<Vec<EntryInfo>> {
    let mut infos = Vec::new();

    for entry in fs::read_dir(dir_buf)?.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };

        if !ctx.filter.hidden && is_hidden(name_str) {
            continue;
        }

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let is_dir = ft.is_dir();
        let is_file = ft.is_file();

        if !is_dir && !is_file {
            continue;
        }

        if is_dir && should_skip(name_str) {
            continue;
        }

        let (size, modified, created) = if is_file && ctx.info.needs_metadata() {
            match entry.metadata() {
                Ok(meta) => (
                    meta.len(),
                    systime_to_epoch(meta.modified().ok()),
                    systime_to_epoch(meta.created().ok()),
                ),
                Err(_) => continue,
            }
        } else {
            (0, 0, 0)
        };

        let dirty = if ctx.info.git && is_file {
            let rel = rel_path(dir_buf, ctx.root, name_str);
            ctx.dirty_files.contains(&rel)
        } else {
            false
        };

        infos.push(EntryInfo {
            name: name_str.to_string(),
            is_dir,
            size,
            modified,
            created,
            dirty,
        });
    }

    sort_entries(&mut infos, ctx.order);
    Ok(infos)
}

fn walk(dir_buf: &mut PathBuf, ctx: &WalkCtx, depth: usize, out: &mut impl Write) -> io::Result<()> {
    let infos = collect_entries(dir_buf.as_path(), ctx)?;

    for info in &infos {
        if info.is_dir {
            let at_limit = depth >= ctx.filter.max_depth;
            if ctx.filter.dirs {
                write_indent(out, depth)?;
                out.write_all(info.name.as_bytes())?;
                if at_limit && ctx.filter.count {
                    dir_buf.push(&info.name);
                    let n = count_files(dir_buf, &ctx.filter)?;
                    dir_buf.pop();
                    writeln!(out, "/ {n}")?;
                } else {
                    writeln!(out, "/")?;
                }
            }
            if !at_limit {
                dir_buf.push(&info.name);
                walk(dir_buf, ctx, depth + 1, out)?;
                dir_buf.pop();
            }
        } else if ctx.filter.files {
            write_file_line(out, depth, info, ctx)?;
        }
    }

    Ok(())
}

fn grep_walk(
    dir_buf: &mut PathBuf,
    ctx: &WalkCtx,
    glob: &Glob,
    depth: usize,
    out: &mut impl Write,
) -> io::Result<bool> {
    let infos = collect_entries(dir_buf.as_path(), ctx)?;
    let mut found = false;

    for info in &infos {
        if info.is_dir {
            let mut buf = Vec::new();
            dir_buf.push(&info.name);
            let has_matches = grep_walk(dir_buf, ctx, glob, depth + 1, &mut buf)?;
            dir_buf.pop();
            if has_matches {
                found = true;
                write_indent(out, depth)?;
                out.write_all(info.name.as_bytes())?;
                writeln!(out, "/")?;
                out.write_all(&buf)?;
            }
        } else if glob.matches(&info.name) {
            found = true;
            write_file_line(out, depth, info, ctx)?;
        }
    }

    Ok(found)
}

fn write_file_line(out: &mut impl Write, depth: usize, info: &EntryInfo, ctx: &WalkCtx) -> io::Result<()> {
    write_indent(out, depth)?;
    if ctx.info.git && info.dirty {
        out.write_all(b"*")?;
    }
    out.write_all(info.name.as_bytes())?;
    if ctx.info.size {
        write!(out, " {}", ctx.unit.format(info.size))?;
    }
    if ctx.info.modified {
        write!(out, " {}", epoch_to_date(info.modified))?;
    }
    if ctx.info.created {
        write!(out, " {}", epoch_to_date(info.created))?;
    }
    writeln!(out)
}

fn sort_entries(entries: &mut [EntryInfo], order: SortOrder) {
    match order {
        SortOrder::Name => entries.sort_by(|a, b| a.name.cmp(&b.name)),
        SortOrder::Size => entries.sort_by(|a, b| b.size.cmp(&a.size)),
        SortOrder::Modified => entries.sort_by(|a, b| b.modified.cmp(&a.modified)),
        SortOrder::Created => entries.sort_by(|a, b| b.created.cmp(&a.created)),
    }
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

fn rel_path(dir: &Path, root: &Path, name: &str) -> String {
    let full = dir.join(name);
    match full.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().into_owned(),
        Err(_) => name.to_string(),
    }
}

fn systime_to_epoch(t: Option<SystemTime>) -> u64 {
    t.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn epoch_to_date(epoch: u64) -> String {
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

fn load_git_dirty(root: &Path, explicit: bool) -> HashSet<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-unormal"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => {
            if explicit {
                eprintln!("warn: not a git repo, -i g ignored");
            }
            return HashSet::new();
        }
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            if line.len() > 3 {
                Some(line[3..].to_string())
            } else {
                None
            }
        })
        .collect()
}
