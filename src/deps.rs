use std::fs;
use std::io::{self, Write};
use std::path::Path;

struct Dep {
    name: String,
    version: String,
    dev: bool,
}

fn strip_version_prefix(v: &str) -> &str {
    let v = v.trim();
    if v.starts_with(">=") || v.starts_with("<=") || v.starts_with("==") || v.starts_with("~=") {
        &v[2..]
    } else if v.starts_with('^') || v.starts_with('~') || v.starts_with('>') || v.starts_with('<') {
        &v[1..]
    } else {
        v
    }
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ── Cargo.toml ──────────────────────────────────────────────────────────────

fn parse_cargo_toml(content: &str) -> Vec<Dep> {
    let mut deps = Vec::new();
    let mut section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Track section headers
        if trimmed.starts_with('[') {
            if let Some(end) = trimmed.find(']') {
                section = trimmed[1..end].trim().to_string();
            }
            continue;
        }

        let is_dep = section == "dependencies";
        let is_dev = section == "dev-dependencies";
        if !is_dep && !is_dev {
            continue;
        }

        // Parse name = "version" or name = { version = "..." ... }
        if let Some(eq_pos) = trimmed.find('=') {
            let name = trimmed[..eq_pos].trim().to_string();
            let value = trimmed[eq_pos + 1..].trim();

            let version = if value.starts_with('"') || value.starts_with('\'') {
                // Simple: name = "version"
                strip_version_prefix(unquote(value)).to_string()
            } else if value.starts_with('{') {
                // Inline table: name = { version = "...", ... }
                extract_version_from_inline_table(value)
            } else {
                continue;
            };

            if !version.is_empty() {
                deps.push(Dep { name, version, dev: is_dev });
            }
        }
    }

    deps
}

fn extract_version_from_inline_table(s: &str) -> String {
    // Look for version = "..." inside braces
    let inner = if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        &s[start + 1..end]
    } else {
        s
    };

    for part in inner.split(',') {
        let part = part.trim();
        if let Some(eq) = part.find('=') {
            let key = part[..eq].trim();
            let val = part[eq + 1..].trim();
            if key == "version" {
                return strip_version_prefix(unquote(val)).to_string();
            }
        }
    }
    String::new()
}

// ── package.json ────────────────────────────────────────────────────────────

fn parse_package_json(content: &str) -> Vec<Dep> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    let mut in_dev_deps = false;
    let mut brace_depth: i32 = 0;
    let mut section_brace_depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Count braces for depth tracking
        let open_count = trimmed.chars().filter(|&c| c == '{').count() as i32;
        let close_count = trimmed.chars().filter(|&c| c == '}').count() as i32;

        // Check for section starts before updating depth
        if trimmed.contains("\"dependencies\"") && trimmed.contains(':') && !trimmed.contains("\"devDependencies\"") {
            in_deps = true;
            in_dev_deps = false;
            // Section starts at the brace on this line or next
            section_brace_depth = brace_depth + open_count;
            brace_depth += open_count - close_count;
            continue;
        }
        if trimmed.contains("\"devDependencies\"") && trimmed.contains(':') {
            in_dev_deps = true;
            in_deps = false;
            section_brace_depth = brace_depth + open_count;
            brace_depth += open_count - close_count;
            continue;
        }

        brace_depth += open_count - close_count;

        // Check if we've left the current section
        if (in_deps || in_dev_deps) && brace_depth < section_brace_depth {
            in_deps = false;
            in_dev_deps = false;
            continue;
        }

        if !in_deps && !in_dev_deps {
            continue;
        }

        // Parse "name": "version" lines
        if let Some((name, version)) = parse_json_kv(trimmed) {
            deps.push(Dep {
                name,
                version: strip_version_prefix(&version).to_string(),
                dev: in_dev_deps,
            });
        }
    }

    deps
}

/// Parse a JSON key-value pair like `"name": "value"` (with optional trailing comma)
fn parse_json_kv(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_end_matches(',');
    // Need at least "k": "v"
    let colon = line.find(':')?;
    let key = line[..colon].trim();
    let val = line[colon + 1..].trim();

    let key = unquote(key);
    let val = unquote(val);

    if key.is_empty() || val.is_empty() || val.starts_with('{') || val.starts_with('[') {
        return None;
    }

    Some((key.to_string(), val.to_string()))
}

// ── pyproject.toml ──────────────────────────────────────────────────────────

fn parse_pyproject_toml(content: &str) -> Vec<Dep> {
    let mut deps = Vec::new();
    let mut section = String::new();
    let mut in_array = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if in_array && trimmed.is_empty() {
                // Empty line might end an implicit array context, but we track ]
            }
            continue;
        }

        // Track section headers
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            if let Some(end) = trimmed.find(']') {
                section = trimmed[1..end].trim().to_string();
                in_array = false;
            }
            continue;
        }

        let is_project_deps = section == "project" && trimmed.starts_with("dependencies");
        let is_optional_dev = section.starts_with("project.optional-dependencies");
        let is_poetry_dev = section == "tool.poetry.group.dev.dependencies"
            || section == "tool.poetry.dev-dependencies";
        let is_poetry_deps = section == "tool.poetry.dependencies";

        // Handle project.dependencies = [ ... ]
        if is_project_deps && trimmed.contains('[') {
            in_array = true;
            // Check if the array is on the same line
            if let Some(bracket) = trimmed.find('[') {
                let after = &trimmed[bracket + 1..];
                if let Some(end) = after.find(']') {
                    // Single-line array
                    let items = &after[..end];
                    for item in items.split(',') {
                        if let Some(dep) = parse_pep508(item.trim(), false) {
                            deps.push(dep);
                        }
                    }
                    in_array = false;
                }
            }
            continue;
        }

        if in_array && section == "project" {
            if trimmed.starts_with(']') {
                in_array = false;
                continue;
            }
            let item = trimmed.trim_end_matches(',');
            if let Some(dep) = parse_pep508(item, false) {
                deps.push(dep);
            }
            continue;
        }

        // Handle optional-dependencies sections (arrays)
        if is_optional_dev {
            if trimmed.contains('[') {
                in_array = true;
                // Check inline array
                if let Some(bracket) = trimmed.find('[') {
                    let after = &trimmed[bracket + 1..];
                    if let Some(end) = after.find(']') {
                        let items = &after[..end];
                        for item in items.split(',') {
                            if let Some(dep) = parse_pep508(item.trim(), true) {
                                deps.push(dep);
                            }
                        }
                        in_array = false;
                    }
                }
                continue;
            }
            if in_array {
                if trimmed.starts_with(']') {
                    in_array = false;
                    continue;
                }
                let item = trimmed.trim_end_matches(',');
                if let Some(dep) = parse_pep508(item, true) {
                    deps.push(dep);
                }
                continue;
            }
        }

        // Handle poetry dependencies (key = "version" format)
        if is_poetry_deps || is_poetry_dev {
            if let Some(eq_pos) = trimmed.find('=') {
                let name = trimmed[..eq_pos].trim();
                if name == "python" {
                    continue;
                }
                let value = trimmed[eq_pos + 1..].trim();
                let version = if value.starts_with('"') || value.starts_with('\'') {
                    strip_version_prefix(unquote(value)).to_string()
                } else if value.starts_with('{') {
                    extract_version_from_inline_table(value)
                } else {
                    continue;
                };
                if !version.is_empty() {
                    deps.push(Dep {
                        name: name.to_string(),
                        version,
                        dev: is_poetry_dev,
                    });
                }
            }
        }
    }

    deps
}

/// Parse a PEP 508 dependency string like `"requests>=2.28"` or `requests>=2.28`
fn parse_pep508(s: &str, dev: bool) -> Option<Dep> {
    let s = unquote(s).trim();
    if s.is_empty() {
        return None;
    }

    // Split at version specifier: >=, <=, ==, ~=, !=, >, <
    let version_ops = [">=", "<=", "==", "~=", "!=", ">", "<"];
    for op in &version_ops {
        if let Some(pos) = s.find(op) {
            let name = s[..pos].trim().to_string();
            let version_part = &s[pos + op.len()..];
            // Take only the first version (before any comma for ranges)
            let version = version_part.split(',').next().unwrap_or("").trim();
            // Remove any extras markers like [extra] from name
            let name = name.split('[').next().unwrap_or("").trim().to_string();
            if !name.is_empty() && !version.is_empty() {
                return Some(Dep { name, version: version.to_string(), dev });
            }
        }
    }

    // No version specifier — just a name
    let name = s.split('[').next().unwrap_or("").split(';').next().unwrap_or("").trim().to_string();
    if !name.is_empty() {
        return Some(Dep { name, version: String::new(), dev });
    }

    None
}

// ── requirements.txt ────────────────────────────────────────────────────────

fn parse_requirements_txt(content: &str) -> Vec<Dep> {
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }

        // Handle inline comments
        let trimmed = if let Some(pos) = trimmed.find('#') {
            trimmed[..pos].trim()
        } else {
            trimmed
        };

        if let Some(dep) = parse_pep508(trimmed, false) {
            deps.push(dep);
        }
    }

    deps
}

// ── go.mod ──────────────────────────────────────────────────────────────────

fn parse_go_mod(content: &str) -> Vec<Dep> {
    let mut deps = Vec::new();
    let mut in_require = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == ")" {
            in_require = false;
            continue;
        }

        if trimmed.starts_with("require (") || trimmed == "require (" {
            in_require = true;
            continue;
        }

        // Single-line require
        if trimmed.starts_with("require ") && !trimmed.contains('(') {
            let rest = trimmed["require ".len()..].trim();
            if let Some(dep) = parse_go_require(rest) {
                deps.push(dep);
            }
            continue;
        }

        if in_require {
            if let Some(dep) = parse_go_require(trimmed) {
                deps.push(dep);
            }
        }
    }

    deps
}

fn parse_go_require(line: &str) -> Option<Dep> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("//") {
        return None;
    }

    // Remove inline comment
    let line = if let Some(pos) = line.find("//") {
        line[..pos].trim()
    } else {
        line
    };

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let module_path = parts[0];
    let version = parts[1].trim_start_matches('v');

    // Use last path segment as short name
    let short_name = module_path.rsplit('/').next().unwrap_or(module_path);

    Some(Dep {
        name: short_name.to_string(),
        version: version.to_string(),
        dev: false,
    })
}

// ── Gemfile ─────────────────────────────────────────────────────────────────

fn parse_gemfile(content: &str) -> Vec<Dep> {
    let mut deps = Vec::new();
    let mut in_dev_group = false;
    let mut group_depth: usize = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Track group blocks
        if trimmed.starts_with("group") && trimmed.contains("do") {
            let is_dev = trimmed.contains(":development") || trimmed.contains(":test");
            if is_dev {
                in_dev_group = true;
            }
            group_depth += 1;
            continue;
        }

        if trimmed == "end" {
            if group_depth > 0 {
                group_depth -= 1;
                if group_depth == 0 {
                    in_dev_group = false;
                }
            }
            continue;
        }

        // Parse gem lines
        if trimmed.starts_with("gem ") || trimmed.starts_with("gem\t") {
            let rest = trimmed[3..].trim();
            if let Some(dep) = parse_gem_line(rest, in_dev_group) {
                deps.push(dep);
            }
        }
    }

    deps
}

fn parse_gem_line(s: &str, dev: bool) -> Option<Dep> {
    // gem 'name', '~> version'  or  gem 'name', '>= version'  or  gem 'name'
    let parts: Vec<&str> = s.splitn(3, ',').collect();
    let name = unquote(parts[0].trim()).to_string();
    if name.is_empty() {
        return None;
    }

    let version = if parts.len() >= 2 {
        let v = unquote(parts[1].trim());
        strip_version_prefix(v).to_string()
    } else {
        String::new()
    };

    Some(Dep { name, version, dev })
}

// ── composer.json ───────────────────────────────────────────────────────────

fn parse_composer_json(content: &str) -> Vec<Dep> {
    let mut deps = Vec::new();
    let mut in_require = false;
    let mut in_require_dev = false;
    let mut brace_depth: i32 = 0;
    let mut section_brace_depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        let open_count = trimmed.chars().filter(|&c| c == '{').count() as i32;
        let close_count = trimmed.chars().filter(|&c| c == '}').count() as i32;

        if trimmed.contains("\"require-dev\"") && trimmed.contains(':') {
            in_require_dev = true;
            in_require = false;
            section_brace_depth = brace_depth + open_count;
            brace_depth += open_count - close_count;
            continue;
        }
        if trimmed.contains("\"require\"") && trimmed.contains(':') && !trimmed.contains("\"require-dev\"") {
            in_require = true;
            in_require_dev = false;
            section_brace_depth = brace_depth + open_count;
            brace_depth += open_count - close_count;
            continue;
        }

        brace_depth += open_count - close_count;

        if (in_require || in_require_dev) && brace_depth < section_brace_depth {
            in_require = false;
            in_require_dev = false;
            continue;
        }

        if !in_require && !in_require_dev {
            continue;
        }

        if let Some((name, version)) = parse_json_kv(trimmed) {
            // Skip php and ext-* entries
            if name == "php" || name.starts_with("ext-") {
                continue;
            }
            deps.push(Dep {
                name,
                version: strip_version_prefix(&version).to_string(),
                dev: in_require_dev,
            });
        }
    }

    deps
}

// ── Output ──────────────────────────────────────────────────────────────────

fn write_deps(deps: &[Dep], indent: &str, out: &mut impl Write) -> io::Result<()> {
    for dep in deps {
        write!(out, "{indent}{}", dep.name)?;
        if !dep.version.is_empty() {
            write!(out, " {}", dep.version)?;
        }
        if dep.dev {
            write!(out, " dev")?;
        }
        writeln!(out)?;
    }
    Ok(())
}

// ── Entry point ─────────────────────────────────────────────────────────────

const MANIFEST_FILES: &[(&str, fn(&str) -> Vec<Dep>)] = &[
    ("Cargo.toml", parse_cargo_toml),
    ("package.json", parse_package_json),
    ("pyproject.toml", parse_pyproject_toml),
    ("requirements.txt", parse_requirements_txt),
    ("go.mod", parse_go_mod),
    ("Gemfile", parse_gemfile),
    ("composer.json", parse_composer_json),
];

pub fn run_deps(root: &Path, out: &mut impl Write) -> io::Result<()> {
    // Collect all (filename, deps) pairs where deps is non-empty
    let mut results: Vec<(&str, Vec<Dep>)> = Vec::new();

    for &(filename, parser) in MANIFEST_FILES {
        let path = root.join(filename);
        if path.is_file() {
            let content = fs::read_to_string(&path)?;
            let deps = parser(&content);
            if !deps.is_empty() {
                results.push((filename, deps));
            }
        }
    }

    match results.len() {
        0 => {}
        1 => {
            let (_, deps) = &results[0];
            write_deps(deps, "", out)?;
        }
        _ => {
            for (filename, deps) in &results {
                writeln!(out, "{filename}")?;
                write_deps(deps, " ", out)?;
            }
        }
    }

    Ok(())
}
