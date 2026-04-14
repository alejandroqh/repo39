use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub fn load_git_dirty(root: &Path, explicit: bool) -> HashSet<String> {
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
        .filter_map(|line| line.get(3..).map(|s| s.to_string()))
        .collect()
}
