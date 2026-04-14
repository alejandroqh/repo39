use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::glob::Glob;
use crate::util::{is_hidden, should_skip};

struct LangPatterns {
    extensions: &'static [&'static str],
    regexes: Vec<Regex>,
}

static LANGUAGES: LazyLock<Vec<LangPatterns>> = LazyLock::new(|| {
    vec![
        // Rust
        lang(&["rs"], &[
            r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)",
            r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)",
            r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?enum\s+(\w+)",
            r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?trait\s+(\w+)",
            r"^[ \t]*impl(?:<[^>]*>)?\s+(\w+)",
        ]),
        // Python
        lang(&["py"], &[
            r"^[ \t]*def\s+(\w+)",
            r"^[ \t]*class\s+(\w+)",
        ]),
        // JavaScript
        lang(&["js", "mjs", "cjs"], &[
            r"^[ \t]*function\s+(\w+)",
            r"^[ \t]*class\s+(\w+)",
            r"^[ \t]*export\s+(?:default\s+)?function\s+(\w+)",
            r"^[ \t]*export\s+(?:default\s+)?class\s+(\w+)",
            r"^[ \t]*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:.*=>|.*\bfunction\b)",
        ]),
        // TypeScript
        lang(&["ts", "tsx"], &[
            r"^[ \t]*function\s+(\w+)",
            r"^[ \t]*class\s+(\w+)",
            r"^[ \t]*export\s+(?:default\s+)?function\s+(\w+)",
            r"^[ \t]*export\s+(?:default\s+)?class\s+(\w+)",
            r"^[ \t]*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:.*=>|.*\bfunction\b)",
            r"^[ \t]*(?:export\s+)?interface\s+(\w+)",
            r"^[ \t]*(?:export\s+)?type\s+(\w+)\s*=",
        ]),
        // Go
        lang(&["go"], &[
            r"^func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)",
            r"^type\s+(\w+)\s+struct\b",
            r"^type\s+(\w+)\s+interface\b",
        ]),
        // Java / Kotlin
        lang(&["java", "kt"], &[
            r"^[ \t]*(?:public\s+|private\s+|protected\s+)?(?:static\s+)?(?:class|interface|enum)\s+(\w+)",
            r"^[ \t]*(?:public\s+|private\s+|protected\s+)?(?:static\s+)?(?:fun|void|int|String|boolean|long)\s+(\w+)",
        ]),
        // Ruby
        lang(&["rb"], &[
            r"^[ \t]*def\s+(\w+)",
            r"^[ \t]*class\s+(\w+)",
            r"^[ \t]*module\s+(\w+)",
        ]),
        // PHP
        lang(&["php"], &[
            r"^[ \t]*(?:public\s+|private\s+|protected\s+)?(?:static\s+)?function\s+(\w+)",
            r"^[ \t]*class\s+(\w+)",
        ]),
        // C/C++
        lang(&["c", "cpp", "cc", "h", "hpp"], &[
            r"^[ \t]*(?:\w+[\w\s\*]*?)\s+(\w+)\s*\(",
        ]),
        // Swift
        lang(&["swift"], &[
            r"^[ \t]*(?:public\s+|private\s+|internal\s+|open\s+)?func\s+(\w+)",
            r"^[ \t]*(?:public\s+|private\s+|internal\s+|open\s+)?class\s+(\w+)",
            r"^[ \t]*(?:public\s+|private\s+|internal\s+|open\s+)?struct\s+(\w+)",
            r"^[ \t]*(?:public\s+|private\s+|internal\s+|open\s+)?protocol\s+(\w+)",
            r"^[ \t]*(?:public\s+|private\s+|internal\s+|open\s+)?enum\s+(\w+)",
        ]),
        // Elixir
        lang(&["ex", "exs"], &[
            r"^[ \t]*def\s+(\w+)",
            r"^[ \t]*defp\s+(\w+)",
            r"^[ \t]*defmodule\s+(\w+)",
        ]),
        // Shell
        lang(&["sh", "bash", "zsh"], &[
            r"^[ \t]*(\w+)\s*\(\)",
            r"^[ \t]*function\s+(\w+)",
        ]),
    ]
});

fn lang(extensions: &'static [&'static str], patterns: &[&str]) -> LangPatterns {
    LangPatterns {
        extensions,
        regexes: patterns.iter().map(|p| Regex::new(p).unwrap()).collect(),
    }
}

fn find_lang(ext: &str) -> Option<&'static LangPatterns> {
    LANGUAGES.iter().find(|l| l.extensions.contains(&ext))
}

fn extract_symbols(path: &Path, lang: &LangPatterns) -> io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut symbols = Vec::new();
    let mut seen = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        for re in &lang.regexes {
            if let Some(caps) = re.captures(&line) {
                if let Some(m) = caps.get(1) {
                    let name = m.as_str().to_string();
                    let prefix = symbol_prefix(&line);
                    let full = format!("{prefix}{name}");
                    if seen.insert(full.clone()) {
                        symbols.push(full);
                    }
                }
                break; // first matching pattern wins for this line
            }
        }
    }

    Ok(symbols)
}

/// Extract a compact keyword prefix from the line. Uses starts_with ordered
/// longest-first within each family to avoid false matches (e.g. "default" vs "def").
fn symbol_prefix(line: &str) -> &'static str {
    let trimmed = line.trim_start();
    for (kw, prefix) in &[
        ("export default function ", "fn "),
        ("export default class ", "class "),
        ("export function ", "fn "),
        ("export class ", "class "),
        ("export interface ", "interface "),
        ("export type ", "type "),
        ("export const ", "const "),
        ("defmodule ", "defmodule "),
        ("defp ", "defp "),
        ("def ", "def "),
        ("pub fn ", "fn "),
        ("fn ", "fn "),
        ("pub struct ", "struct "),
        ("struct ", "struct "),
        ("pub enum ", "enum "),
        ("enum ", "enum "),
        ("pub trait ", "trait "),
        ("trait ", "trait "),
        ("impl ", "impl "),
        ("interface ", "interface "),
        ("type ", "type "),
        ("class ", "class "),
        ("module ", "module "),
        ("protocol ", "protocol "),
        ("func ", "fn "),
        ("function ", "fn "),
        ("const ", "const "),
    ] {
        if trimmed.starts_with(kw) {
            return prefix;
        }
    }
    ""
}

fn collect_files(
    dir: &Path,
    root: &Path,
    depth: usize,
    max_depth: usize,
    files: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)?.filter_map(|e| e.ok()) {
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
            if depth < max_depth {
                collect_files(&entry.path(), root, depth + 1, max_depth, files)?;
            }
        } else if ft.is_file() {
            let path = entry.path();
            let rel = match path.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().into_owned(),
                Err(_) => continue,
            };
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if find_lang(ext).is_some() {
                    files.push((rel, path));
                }
            }
        }
    }

    Ok(())
}

pub fn run_map(
    root: &Path,
    depth: usize,
    limit: usize,
    grep: Option<&str>,
    out: &mut impl Write,
) -> io::Result<()> {
    let mut files = Vec::new();
    collect_files(root, root, 0, depth, &mut files)?;

    files.sort_by(|a, b| a.0.cmp(&b.0));

    let glob = grep.map(Glob::compile);

    // Collect files that have symbols (after grep filtering)
    let mut results: Vec<(&str, Vec<String>)> = Vec::new();
    for (rel, path) in &files {
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => continue,
        };
        let lang = match find_lang(ext) {
            Some(l) => l,
            None => continue,
        };
        let symbols = extract_symbols(path, lang)?;
        if symbols.is_empty() {
            continue;
        }

        let filtered: Vec<String> = match &glob {
            Some(g) => symbols.into_iter().filter(|s| {
                let name = s.split_whitespace().last().unwrap_or(s);
                g.matches(name)
            }).collect(),
            None => symbols,
        };

        if !filtered.is_empty() {
            results.push((rel, filtered));
        }
    }

    // Output with tree-style indentation: dir/ → file → symbols
    let mut last_dir = String::new();
    for (rel, symbols) in &results {
        let (dir, filename) = match rel.rfind('/') {
            Some(pos) => (&rel[..pos], &rel[pos + 1..]),
            None => ("", *rel),
        };

        // Emit directory headers when directory changes
        if !dir.is_empty() && dir != last_dir {
            // Emit nested dir components
            let parts: Vec<&str> = dir.split('/').collect();
            let last_parts: Vec<&str> = if last_dir.is_empty() {
                Vec::new()
            } else {
                last_dir.split('/').collect()
            };
            // Find where paths diverge
            let common = parts.iter().zip(last_parts.iter()).take_while(|(a, b)| a == b).count();
            for (i, part) in parts.iter().enumerate().skip(common) {
                let indent = " ".repeat(i);
                writeln!(out, "{indent}{part}/")?;
            }
            last_dir = dir.to_string();
        }

        let file_depth = if dir.is_empty() { 0 } else { dir.matches('/').count() + 1 };
        let file_indent = " ".repeat(file_depth);
        let sym_indent = " ".repeat(file_depth + 1);

        writeln!(out, "{file_indent}{filename}")?;
        if limit > 0 && symbols.len() > limit {
            for sym in &symbols[..limit] {
                writeln!(out, "{sym_indent}{sym}")?;
            }
            let hidden = symbols.len() - limit;
            writeln!(out, "{sym_indent}...+{hidden}")?;
        } else {
            for sym in symbols {
                writeln!(out, "{sym_indent}{sym}")?;
            }
        }
    }

    Ok(())
}
