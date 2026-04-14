use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use crate::map::{extract_symbols, extract_symbols_from_bytes, find_lang};

pub fn run_review(
    root: &Path,
    ref_spec: Option<&str>,
    out: &mut impl Write,
) -> io::Result<()> {
    let ref_spec = ref_spec.unwrap_or("HEAD~1");

    // Get list of changed files
    let output = Command::new("git")
        .args(["diff", "--name-only", ref_spec, "--"])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        writeln!(out, "error: git diff failed: {}", err.trim())?;
        return Ok(());
    }

    let changed_files: Vec<&str> = std::str::from_utf8(&output.stdout)
        .unwrap_or("")
        .lines()
        .filter(|l| !l.is_empty())
        .take(20) // limit to 20 files
        .collect();

    if changed_files.is_empty() {
        return Ok(());
    }

    for file_path in &changed_files {
        let ext = match Path::new(file_path).extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => continue,
        };
        let lang = match find_lang(ext) {
            Some(l) => l,
            None => continue,
        };

        // Get current symbols
        let full_path = root.join(file_path);
        let current_syms = if full_path.exists() {
            extract_symbols(&full_path, lang).unwrap_or_default()
        } else {
            Vec::new() // file was deleted
        };

        // Get old symbols via git show
        let old_output = Command::new("git")
            .args(["show", &format!("{ref_spec}:{file_path}")])
            .current_dir(root)
            .output();

        let old_syms = match old_output {
            Ok(out) if out.status.success() => {
                extract_symbols_from_bytes(&out.stdout, lang)
            }
            _ => Vec::new(), // file is new
        };

        // Build maps: stripped symbol_name → line_number
        let current_map: HashMap<&str, usize> = current_syms.iter()
            .map(|(s, line)| (strip_vis(s), *line))
            .collect();
        let old_map: HashMap<&str, usize> = old_syms.iter()
            .map(|(s, line)| (strip_vis(s), *line))
            .collect();

        // Compute diffs (using stripped names for comparison and output)
        let mut added: Vec<(&str, usize)> = Vec::new();
        let mut removed: Vec<&str> = Vec::new();
        let mut modified: Vec<(&str, usize)> = Vec::new();

        for (sym, line) in &current_syms {
            let key = strip_vis(sym);
            if !old_map.contains_key(key) {
                added.push((key, *line));
            } else if old_map[key] != current_map[key] {
                modified.push((key, *line));
            }
        }
        for (sym, _) in &old_syms {
            let key = strip_vis(sym);
            if !current_map.contains_key(key) {
                removed.push(key);
            }
        }

        if added.is_empty() && removed.is_empty() && modified.is_empty() {
            continue;
        }

        writeln!(out, "{file_path}")?;
        for (sym, line) in &added {
            writeln!(out, " +{sym}:{line}")?;
        }
        for sym in &removed {
            writeln!(out, " -{sym}")?;
        }
        for (sym, line) in &modified {
            writeln!(out, " ~{sym}:{line}")?;
        }
    }

    Ok(())
}

fn strip_vis(sym: &str) -> &str {
    sym.strip_prefix('+').unwrap_or(sym)
}
