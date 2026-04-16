use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::glob::Glob;
use crate::util::{is_hidden, should_skip};

pub fn run_search(
    root: &Path,
    pattern: &str,
    is_regex: bool,
    context: usize,
    max_results: usize,
    file_glob: Option<&str>,
    out: &mut impl Write,
) -> io::Result<()> {
    let matcher = if is_regex {
        Matcher::Regex(Regex::new(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?)
    } else {
        Matcher::Literal(pattern.to_string())
    };

    let glob = file_glob.map(Glob::compile);
    let mut files = Vec::new();
    collect_searchable_files(root, root, &glob, &mut files)?;
    files.sort();

    let mut total_matches = 0usize;
    let limit = if max_results == 0 { usize::MAX } else { max_results };
    let mut prev_file = false;

    for (rel, path) in &files {
        if total_matches >= limit {
            break;
        }
        let matches = match search_file(path, &matcher, context) {
            Ok(m) => m,
            Err(_) => continue, // binary/unreadable
        };
        if matches.is_empty() {
            continue;
        }
        if prev_file {
            writeln!(out, "--")?;
        }
        prev_file = true;
        for group in &matches {
            if total_matches >= limit {
                break;
            }
            for &(line_num, ref line, is_match) in group {
                if is_match {
                    total_matches += 1;
                }
                if is_match || context > 0 {
                    writeln!(out, "{rel}:{line_num} {line}")?;
                }
            }
        }
    }

    Ok(())
}

enum Matcher {
    Literal(String),
    Regex(Regex),
}

impl Matcher {
    fn is_match(&self, line: &str) -> bool {
        match self {
            Matcher::Literal(s) => line.contains(s.as_str()),
            Matcher::Regex(re) => re.is_match(line),
        }
    }
}

type MatchGroup = Vec<(usize, String, bool)>; // (line_num, content, is_match)

fn search_file(path: &Path, matcher: &Matcher, context: usize) -> io::Result<Vec<MatchGroup>> {
    // Read file, bail if binary (contains NUL bytes in first 512 bytes)
    let content = fs::read(path)?;
    if content.get(..512.min(content.len())).map_or(false, |b| b.contains(&0)) {
        return Ok(Vec::new());
    }

    let lines: Vec<String> = content.lines()
        .take(50_000)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    if lines.is_empty() {
        return Ok(Vec::new());
    }

    // Find matching line indices
    let match_set: HashSet<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| matcher.is_match(l))
        .map(|(i, _)| i)
        .collect();

    if match_set.is_empty() {
        return Ok(Vec::new());
    }

    let mut match_indices: Vec<usize> = match_set.iter().copied().collect();
    match_indices.sort_unstable();

    // Group matches with context
    let mut groups: Vec<MatchGroup> = Vec::new();
    let mut current_group: MatchGroup = Vec::new();
    let mut current_end = 0usize;

    for &idx in &match_indices {
        let start = idx.saturating_sub(context);
        let end = (idx + context + 1).min(lines.len());

        if !current_group.is_empty() && start > current_end {
            groups.push(std::mem::take(&mut current_group));
        }

        let actual_start = if current_group.is_empty() { start } else { current_end };
        for i in actual_start..end {
            let is_match = match_set.contains(&i);
            current_group.push((i + 1, lines[i].clone(), is_match));
        }
        current_end = end;
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    Ok(groups)
}

fn collect_searchable_files(
    dir: &Path,
    root: &Path,
    glob: &Option<Glob>,
    files: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };

        if is_hidden(name_str) {
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
            collect_searchable_files(&entry.path(), root, glob, files)?;
        } else if ft.is_file() {
            if let Some(g) = glob {
                if !g.matches(name_str) {
                    continue;
                }
            }
            let path = entry.path();
            let rel = match path.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().into_owned(),
                Err(_) => continue,
            };
            files.push((rel, path));
        }
    }

    Ok(())
}

