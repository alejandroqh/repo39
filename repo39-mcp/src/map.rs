use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::glob::Glob;
use crate::util::{is_hidden, should_skip};

pub(crate) struct LangPatterns {
    pub(crate) extensions: &'static [&'static str],
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
        // Dart
        lang(&["dart"], &[
            r"^[ \t]*class\s+(\w+)",
            r"^[ \t]*enum\s+(\w+)",
            r"^[ \t]*mixin\s+(\w+)",
            r"^[ \t]*extension\s+(\w+)",
            r"^[ \t]*typedef\s+(\w+)",
            r"^[ \t]*(?:static\s+)?(?:Future|void|String|int|double|bool|List|Map|Set|dynamic|Widget|State|Color|BuildContext)(?:<[^>]*>)?\s+(\w+)\s*\(",
            r"^[ \t]*(?:static\s+)?(?:Future|void|String|int|double|bool|List|Map|Set|dynamic|Widget|State|Color|BuildContext)(?:<[^>]*>)?\s+get\s+(\w+)",
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

pub(crate) fn find_lang(ext: &str) -> Option<&'static LangPatterns> {
    LANGUAGES.iter().find(|l| l.extensions.contains(&ext))
}

pub(crate) fn extract_symbols(path: &Path, lang: &LangPatterns) -> io::Result<Vec<(String, usize)>> {
    let file = fs::File::open(path)?;
    Ok(extract_symbols_inner(BufReader::new(file), lang))
}

pub(crate) fn extract_symbols_from_bytes(content: &[u8], lang: &LangPatterns) -> Vec<(String, usize)> {
    extract_symbols_inner(content, lang)
}

fn extract_symbols_inner(reader: impl BufRead, lang: &LangPatterns) -> Vec<(String, usize)> {
    let mut symbols = Vec::new();
    let mut seen = HashSet::new();
    let is_go = lang.extensions.contains(&"go");

    for (idx, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(_) => return symbols, // binary file or encoding error — stop
        };
        for re in &lang.regexes {
            if let Some(caps) = re.captures(&line) {
                if let Some(m) = caps.get(1) {
                    let name = m.as_str();
                    if name.len() < 2 {
                        break; // skip single-char symbols (noise)
                    }
                    let (prefix, mut is_pub) = symbol_prefix(&line);
                    if is_go && is_go_exported(name) {
                        is_pub = true;
                    }
                    let vis = if is_pub { "+" } else { "" };
                    let full = format!("{vis}{prefix}{name}");
                    if seen.insert(full.clone()) {
                        symbols.push((full, idx + 1));
                    }
                }
                break; // first matching pattern wins for this line
            }
        }
    }

    symbols
}

/// Extract a compact keyword prefix and visibility from the line.
/// Returns (prefix, is_public). Uses starts_with ordered longest-first
/// within each family to avoid false matches (e.g. "default" vs "def").
fn symbol_prefix(line: &str) -> (&'static str, bool) {
    let trimmed = line.trim_start();
    // (keyword, prefix, is_public)
    for &(kw, prefix, vis) in &[
        ("export default function ", "fn ", true),
        ("export default class ", "class ", true),
        ("export function ", "fn ", true),
        ("export class ", "class ", true),
        ("export interface ", "interface ", true),
        ("export type ", "type ", true),
        ("export const ", "const ", true),
        ("defmodule ", "defmodule ", false),
        ("defp ", "defp ", false),
        ("def ", "def ", false),
        ("pub(crate) fn ", "fn ", false),
        ("pub fn ", "fn ", true),
        ("fn ", "fn ", false),
        ("pub(crate) struct ", "struct ", false),
        ("pub struct ", "struct ", true),
        ("struct ", "struct ", false),
        ("pub(crate) enum ", "enum ", false),
        ("pub enum ", "enum ", true),
        ("enum ", "enum ", false),
        ("pub(crate) trait ", "trait ", false),
        ("pub trait ", "trait ", true),
        ("trait ", "trait ", false),
        ("impl ", "impl ", false),
        ("interface ", "interface ", false),
        ("type ", "type ", false),
        ("class ", "class ", false),
        ("mixin ", "mixin ", false),
        ("extension ", "extension ", false),
        ("typedef ", "typedef ", false),
        ("public static void ", "fn ", true),
        ("public static ", "fn ", true),
        ("public void ", "fn ", true),
        ("public class ", "class ", true),
        ("public interface ", "interface ", true),
        ("public enum ", "enum ", true),
        ("private ", "fn ", false),
        ("protected ", "fn ", false),
        ("static void ", "fn ", false),
        ("static Future", "fn ", false),
        ("void ", "fn ", false),
        ("Future", "fn ", false),
        ("String ", "fn ", false),
        ("int ", "fn ", false),
        ("double ", "fn ", false),
        ("bool ", "fn ", false),
        ("List", "fn ", false),
        ("Map", "fn ", false),
        ("Widget ", "fn ", false),
        ("dynamic ", "fn ", false),
        ("State", "fn ", false),
        ("open func ", "fn ", true),
        ("open class ", "class ", true),
        ("open struct ", "struct ", true),
        ("open protocol ", "protocol ", true),
        ("open enum ", "enum ", true),
        ("public func ", "fn ", true),
        ("public struct ", "struct ", true),
        ("public protocol ", "protocol ", true),
        ("module ", "module ", false),
        ("protocol ", "protocol ", false),
        ("func ", "fn ", false),
        ("function ", "fn ", false),
        ("const ", "const ", false),
    ] {
        if trimmed.starts_with(kw) {
            return (prefix, vis);
        }
    }
    ("", false)
}

/// For Go: exported symbols start with uppercase letter.
fn is_go_exported(name: &str) -> bool {
    name.chars().next().map_or(false, |c| c.is_uppercase())
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
    let mut results: Vec<(&str, Vec<(String, usize)>)> = Vec::new();
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

        let filtered: Vec<(String, usize)> = match &glob {
            Some(g) => symbols.into_iter().filter(|(s, _)| {
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
            for (sym, line) in &symbols[..limit] {
                writeln!(out, "{sym_indent}{sym}:{line}")?;
            }
            let hidden = symbols.len() - limit;
            writeln!(out, "{sym_indent}...+{hidden}")?;
        } else {
            for (sym, line) in symbols {
                writeln!(out, "{sym_indent}{sym}:{line}")?;
            }
        }
    }

    Ok(())
}
