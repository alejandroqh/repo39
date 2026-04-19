//! File outlining: split a file into navigable sections (markdown headings,
//! code definitions, or fixed-size chunks) so large files can be previewed or
//! addressed by name instead of dumping the whole body.
//!
//! Ported from files39's internal `outline` module so `repo39::read` can
//! offer the same selector-based partial reads without a cross-crate dep.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub byte_size: usize,
}

pub fn estimate_tokens(bytes: usize) -> u64 {
    (bytes as u64).div_ceil(4)
}

pub fn outline(path: &Path, content: &str) -> Vec<Section> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let lines: Vec<&str> = content.lines().collect();

    if matches!(ext.as_str(), "md" | "markdown" | "rst") {
        outline_markdown(&lines)
    } else if let Some(patterns) = code_patterns(&ext) {
        let sections = outline_code(&lines, patterns);
        if sections.is_empty() {
            outline_chunks(&lines, 200)
        } else {
            sections
        }
    } else {
        outline_chunks(&lines, 200)
    }
}

fn outline_markdown(lines: &[&str]) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current: Option<(String, usize, usize)> = None;

    for (i, line) in lines.iter().enumerate() {
        let line_no = i + 1;
        if line.starts_with('#') {
            let level = line.bytes().take_while(|&b| b == b'#').count();
            if level <= 6 {
                let title = line[level..].trim();
                if !title.is_empty() {
                    if let Some((name, start, bytes)) = current.take() {
                        sections.push(Section {
                            name,
                            start_line: start,
                            end_line: i,
                            byte_size: bytes,
                        });
                    }
                    current = Some((
                        format!("{} {}", "#".repeat(level), title),
                        line_no,
                        line.len() + 1,
                    ));
                    continue;
                }
            }
        }
        if let Some((_, _, ref mut bytes)) = current {
            *bytes += line.len() + 1;
        }
    }
    if let Some((name, start, bytes)) = current {
        sections.push(Section {
            name,
            start_line: start,
            end_line: lines.len(),
            byte_size: bytes,
        });
    }
    sections
}

struct CodePat {
    prefix: &'static str,
    re: Regex,
}

fn code_patterns(ext: &str) -> Option<&'static [CodePat]> {
    static RS: LazyLock<Vec<CodePat>> = LazyLock::new(|| {
        vec![
            pat("fn", r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)"),
            pat("struct", r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)"),
            pat("enum", r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?enum\s+(\w+)"),
            pat("trait", r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?trait\s+(\w+)"),
            pat("impl", r"^[ \t]*impl(?:<[^>]*>)?\s+(\w+)"),
        ]
    });
    static PY: LazyLock<Vec<CodePat>> = LazyLock::new(|| {
        vec![
            pat("def", r"^[ \t]*(?:async\s+)?def\s+(\w+)"),
            pat("class", r"^[ \t]*class\s+(\w+)"),
        ]
    });
    static JS: LazyLock<Vec<CodePat>> = LazyLock::new(|| {
        vec![
            pat("function", r"^[ \t]*(?:export\s+(?:default\s+)?)?(?:async\s+)?function\s+(\w+)"),
            pat("class", r"^[ \t]*(?:export\s+(?:default\s+)?)?class\s+(\w+)"),
            pat("const", r"^[ \t]*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:.*=>|.*\bfunction\b)"),
        ]
    });
    static TS: LazyLock<Vec<CodePat>> = LazyLock::new(|| {
        vec![
            pat("function", r"^[ \t]*(?:export\s+(?:default\s+)?)?(?:async\s+)?function\s+(\w+)"),
            pat("class", r"^[ \t]*(?:export\s+(?:default\s+)?)?class\s+(\w+)"),
            pat("interface", r"^[ \t]*(?:export\s+)?interface\s+(\w+)"),
            pat("type", r"^[ \t]*(?:export\s+)?type\s+(\w+)\s*="),
        ]
    });
    static GO: LazyLock<Vec<CodePat>> = LazyLock::new(|| {
        vec![
            pat("func", r"^func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)"),
            pat("type", r"^type\s+(\w+)\s+(?:struct|interface)\b"),
        ]
    });
    static RB: LazyLock<Vec<CodePat>> = LazyLock::new(|| {
        vec![
            pat("def", r"^[ \t]*def\s+(\w+)"),
            pat("class", r"^[ \t]*class\s+(\w+)"),
            pat("module", r"^[ \t]*module\s+(\w+)"),
        ]
    });

    match ext {
        "rs" => Some(&RS),
        "py" => Some(&PY),
        "js" | "mjs" | "cjs" | "jsx" => Some(&JS),
        "ts" | "tsx" => Some(&TS),
        "go" => Some(&GO),
        "rb" => Some(&RB),
        _ => None,
    }
}

fn pat(prefix: &'static str, re: &str) -> CodePat {
    CodePat {
        prefix,
        re: Regex::new(re).unwrap(),
    }
}

fn outline_code(lines: &[&str], patterns: &[CodePat]) -> Vec<Section> {
    let mut starts: Vec<(usize, String)> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for p in patterns {
            if let Some(c) = p.re.captures(line)
                && let Some(name) = c.get(1)
            {
                starts.push((i, format!("{} {}", p.prefix, name.as_str())));
                break;
            }
        }
    }
    if starts.is_empty() {
        return Vec::new();
    }
    let mut sections = Vec::with_capacity(starts.len());
    for (idx, (start_idx, name)) in starts.iter().enumerate() {
        let end_idx = starts.get(idx + 1).map(|(s, _)| *s).unwrap_or(lines.len());
        let bytes: usize = lines[*start_idx..end_idx]
            .iter()
            .map(|l| l.len() + 1)
            .sum();
        sections.push(Section {
            name: name.clone(),
            start_line: start_idx + 1,
            end_line: end_idx,
            byte_size: bytes,
        });
    }
    sections
}

fn outline_chunks(lines: &[&str], chunk_size: usize) -> Vec<Section> {
    if lines.is_empty() {
        return Vec::new();
    }
    let total = lines.len();
    let mut sections = Vec::new();
    let mut start = 0;
    while start < total {
        let end = (start + chunk_size).min(total);
        let bytes: usize = lines[start..end].iter().map(|l| l.len() + 1).sum();
        sections.push(Section {
            name: format!("lines {}-{}", start + 1, end),
            start_line: start + 1,
            end_line: end,
            byte_size: bytes,
        });
        start = end;
    }
    sections
}
