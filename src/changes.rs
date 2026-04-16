use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct FileChange {
    path: String,
    last_modified: u64,
    insertions: u64,
    deletions: u64,
    is_new: bool,
    is_deleted: bool,
}

fn format_time_ago(epoch: u64, now: u64) -> String {
    let diff = now.saturating_sub(epoch);
    let minutes = diff / 60;
    let hours = diff / 3600;
    let days = diff / 86400;
    let weeks = diff / 604800;
    let months = diff / 2592000;
    let years = diff / 31536000;

    if years > 0 {
        format!("{years}y")
    } else if months > 0 {
        format!("{months}M")
    } else if weeks > 0 {
        format!("{weeks}w")
    } else if days > 0 {
        format!("{days}d")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{minutes}m")
    }
}

/// Normalize git rename paths like `src/{old => new}.rs` or `{old => new}/file.rs`
fn normalize_path(raw: &str) -> String {
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.find('}') {
            let prefix = &raw[..start];
            let suffix = &raw[end + 1..];
            let inner = &raw[start + 1..end];
            if let Some((_old, new)) = inner.split_once(" => ") {
                return format!("{prefix}{new}{suffix}");
            }
        }
    }
    raw.to_string()
}

fn parse_git_log(root: &Path) -> Vec<FileChange> {
    // Get numstat with timestamps
    let numstat_out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--pretty=format:%at", "--numstat", "-n", "100"])
        .output();

    let numstat_out = match numstat_out {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let numstat_str = String::from_utf8_lossy(&numstat_out.stdout);

    // Track per-file aggregates: (most_recent_ts, total_ins, total_del)
    let mut files: HashMap<String, (u64, u64, u64)> = HashMap::new();
    let mut current_ts: u64 = 0;

    for line in numstat_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try timestamp line (just a number)
        if let Ok(ts) = line.parse::<u64>() {
            current_ts = ts;
            continue;
        }

        // Try numstat line: insertions\tdeletions\tpath
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() == 3 {
            // Binary files show "-" for insertions/deletions — skip
            let ins = match parts[0].parse::<u64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let del = match parts[1].parse::<u64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = normalize_path(parts[2]);

            let entry = files.entry(path).or_insert((0, 0, 0));
            if current_ts > entry.0 {
                entry.0 = current_ts;
            }
            entry.1 += ins;
            entry.2 += del;
        }
    }

    // Get added/deleted files in one command
    let status_out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--pretty=format:%at", "--diff-filter=AD", "--name-status", "-n", "100"])
        .output();

    let mut added_files: HashMap<String, u64> = HashMap::new();
    let mut deleted_files: HashMap<String, u64> = HashMap::new();
    if let Ok(o) = status_out {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout);
            let mut ts: u64 = 0;
            for line in s.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(t) = line.parse::<u64>() {
                    ts = t;
                    continue;
                }
                if let Some(path) = line.strip_prefix("A\t") {
                    added_files.insert(path.to_string(), ts);
                } else if let Some(path) = line.strip_prefix("D\t") {
                    deleted_files.insert(path.to_string(), ts);
                }
            }
        }
    }

    let mut changes: Vec<FileChange> = files
        .into_iter()
        .map(|(path, (ts, ins, del))| {
            let is_new = added_files.contains_key(&path);
            let is_deleted = deleted_files.contains_key(&path);
            FileChange {
                path,
                last_modified: ts,
                insertions: ins,
                deletions: del,
                is_new,
                is_deleted,
            }
        })
        .collect();

    // Sort by most recent first
    changes.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    // Limit to 50 entries
    changes.truncate(50);

    changes
}

fn write_change(c: &FileChange, time_prefix: Option<&str>, out: &mut impl Write) -> io::Result<()> {
    if let Some(ago) = time_prefix {
        write!(out, "{ago} ")?;
    }
    write!(out, "{}", c.path)?;
    if c.insertions > 0 { write!(out, " +{}", c.insertions)?; }
    if c.deletions > 0 { write!(out, " -{}", c.deletions)?; }
    if c.is_new { write!(out, " new")?; }
    else if c.is_deleted { write!(out, " del")?; }
    writeln!(out)
}

#[allow(dead_code)]
pub fn run_changes_branch(root: &Path, branch: &str, out: &mut impl Write) -> io::Result<()> {
    let diff_out = Command::new("git")
        .arg("-C").arg(root)
        .args(["diff", "--numstat", branch])
        .output()?;

    if !diff_out.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "git diff failed"));
    }

    let diff_str = String::from_utf8_lossy(&diff_out.stdout);
    let mut files: Vec<FileChange> = Vec::new();

    for line in diff_str.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() == 3 {
            let ins = parts[0].parse::<u64>().unwrap_or(0);
            let del = parts[1].parse::<u64>().unwrap_or(0);
            let path = normalize_path(parts[2]);
            files.push(FileChange {
                path,
                last_modified: 0,
                insertions: ins,
                deletions: del,
                is_new: false,
                is_deleted: false,
            });
        }
    }

    let ad_out = Command::new("git")
        .arg("-C").arg(root)
        .args(["diff", "--diff-filter=AD", "--name-status", branch])
        .output()?;

    if ad_out.status.success() {
        let ad_str = String::from_utf8_lossy(&ad_out.stdout);
        for line in ad_str.lines() {
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.len() == 2 {
                let path = parts[1].to_string();
                if let Some(f) = files.iter_mut().find(|f| f.path == path) {
                    match parts[0] {
                        "A" => f.is_new = true,
                        "D" => f.is_deleted = true,
                        _ => {}
                    }
                }
            }
        }
    }

    files.sort_by(|a, b| b.insertions.cmp(&a.insertions));
    files.truncate(50);

    for c in &files {
        write_change(c, None, out)?;
    }

    Ok(())
}

pub fn run_changes(root: &Path, out: &mut impl Write) -> io::Result<()> {
    let check = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--git-dir"])
        .output();

    match check {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("warn: not a git repo");
            return Ok(());
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let changes = parse_git_log(root);

    for c in &changes {
        let ago = format_time_ago(c.last_modified, now);
        write_change(c, Some(&ago), out)?;
    }

    Ok(())
}
