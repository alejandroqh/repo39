use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config::{InfoFlags, ShowFilter, SizeUnit, SortOrder};
use crate::glob::Glob;
use crate::util::{
    epoch_to_date, is_hidden, rel_path, should_skip, systime_to_epoch, write_indent,
};

pub struct WalkCtx<'a> {
    pub root: &'a Path,
    pub filter: ShowFilter,
    pub order: SortOrder,
    pub info: InfoFlags,
    pub unit: SizeUnit,
    pub limit: usize,
    pub dirty_files: HashSet<String>,
}

struct EntryInfo {
    name: String,
    is_dir: bool,
    size: u64,
    modified: u64,
    created: u64,
    dirty: bool,
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

pub fn walk(dir_buf: &mut PathBuf, ctx: &WalkCtx, depth: usize, out: &mut impl Write) -> io::Result<()> {
    let infos = collect_entries(dir_buf.as_path(), ctx)?;

    let apply_limit = ctx.limit > 0 && depth > 0;
    let (total_files, total_dirs) = if apply_limit {
        infos.iter().fold((0usize, 0usize), |(f, d), i| {
            if i.is_dir { (f, d + 1) } else { (f + 1, d) }
        })
    } else {
        (0, 0)
    };

    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut truncated = false;

    for info in &infos {
        if info.is_dir {
            dir_count += 1;
            if apply_limit && dir_count > ctx.limit {
                truncated = true;
                continue;
            }
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
            file_count += 1;
            if apply_limit && file_count > ctx.limit {
                truncated = true;
                continue;
            }
            write_file_line(out, depth, info, ctx)?;
        }
    }

    if truncated {
        let hidden_files = total_files.saturating_sub(ctx.limit);
        let hidden_dirs = total_dirs.saturating_sub(ctx.limit);
        let total_hidden = hidden_files + hidden_dirs;
        if total_hidden > 0 {
            write_indent(out, depth)?;
            writeln!(out, "...+{total_hidden}")?;
        }
    }

    Ok(())
}

pub fn grep_walk(
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

fn count_files(dir_buf: &mut PathBuf, filter: &ShowFilter) -> io::Result<usize> {
    let mut total = 0;
    let entries = match fs::read_dir(dir_buf.as_path()) {
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
                dir_buf.push(name_str);
                total += count_files(dir_buf, filter)?;
                dir_buf.pop();
            }
        } else if ft.is_file() {
            total += 1;
        }
    }

    Ok(total)
}
