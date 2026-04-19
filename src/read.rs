//! Single-file read with optional selector-based partial reads.
//!
//! Mirrors files39's `run_read` but writes into the `&mut impl Write` sink
//! used by every other repo39 operation, so the agent39 wrapper can buffer
//! it uniformly.
//!
//! Selector forms:
//! - `lines:N-M` — inclusive line range.
//! - `<section name>` — matches an outlined section (exact, then substring).
//!
//! If no selector is supplied and the file exceeds `max_tokens`,
//! `show_outline_first` returns an outline instead of the full body so the
//! caller can drill in with a selector on a follow-up call.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::outline::{Section, estimate_tokens, outline};

pub fn run_read(
    path: &Path,
    selector: Option<&str>,
    max_tokens: u64,
    show_outline_first: bool,
    out: &mut impl Write,
) -> std::io::Result<()> {
    let content = fs::read_to_string(path)?;
    let total_tokens = estimate_tokens(content.len());
    let total_lines = content.lines().count();
    let sections = outline(path, &content);

    if let Some(sel) = selector {
        if let Some(s) = parse_line_range(sel, total_lines) {
            return write_section(out, path, &content, &s);
        }
        if let Some(s) = find_section(&sections, sel) {
            return write_section(out, path, &content, s);
        }
        writeln!(out, "selector '{sel}' not found in {}", path.display())?;
        writeln!(out)?;
        return write_outline(out, path, &sections, total_tokens);
    }

    if show_outline_first && total_tokens > max_tokens && sections.len() > 1 {
        return write_outline(out, path, &sections, total_tokens);
    }

    write_full(out, path, &content, total_tokens)
}

fn find_section<'a>(sections: &'a [Section], sel: &str) -> Option<&'a Section> {
    let needle = sel.to_lowercase();
    sections
        .iter()
        .find(|s| s.name.to_lowercase() == needle)
        .or_else(|| sections.iter().find(|s| s.name.to_lowercase().contains(&needle)))
}

fn parse_line_range(sel: &str, total_lines: usize) -> Option<Section> {
    let body = sel
        .strip_prefix("lines:")
        .or_else(|| sel.strip_prefix("lines "))?;
    let (a, b) = body.split_once('-')?;
    let start: usize = a.trim().parse().ok()?;
    let end: usize = b.trim().parse().ok()?;
    if start == 0 || end < start {
        return None;
    }
    let end = end.min(total_lines.max(1));
    Some(Section {
        name: format!("lines {start}-{end}"),
        start_line: start,
        end_line: end,
        byte_size: 0,
    })
}

fn write_outline(
    out: &mut impl Write,
    path: &Path,
    sections: &[Section],
    total_tokens: u64,
) -> std::io::Result<()> {
    writeln!(out, "{} (~{total_tokens} tokens)", path.display())?;
    for s in sections {
        writeln!(
            out,
            " {} [{}-{}] ~{}t",
            s.name,
            s.start_line,
            s.end_line,
            estimate_tokens(s.byte_size)
        )?;
    }
    writeln!(out)?;
    writeln!(
        out,
        "file too large; re-call with selector=\"<name>\" or selector=\"lines:N-M\""
    )?;
    Ok(())
}

fn write_section(
    out: &mut impl Write,
    path: &Path,
    content: &str,
    s: &Section,
) -> std::io::Result<()> {
    writeln!(
        out,
        "{} [{} {}-{}]",
        path.display(),
        s.name,
        s.start_line,
        s.end_line
    )?;
    let take = s.end_line.saturating_sub(s.start_line) + 1;
    write_numbered_lines(out, content, s.start_line, take)
}

fn write_full(
    out: &mut impl Write,
    path: &Path,
    content: &str,
    total_tokens: u64,
) -> std::io::Result<()> {
    writeln!(out, "{} (~{total_tokens} tokens)", path.display())?;
    write_numbered_lines(out, content, 1, usize::MAX)
}

fn write_numbered_lines(
    out: &mut impl Write,
    content: &str,
    start_line: usize,
    take: usize,
) -> std::io::Result<()> {
    for (i, line) in content
        .lines()
        .enumerate()
        .skip(start_line.saturating_sub(1))
        .take(take)
    {
        writeln!(out, "{:>5}: {line}", i + 1)?;
    }
    Ok(())
}
