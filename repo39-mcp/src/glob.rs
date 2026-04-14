/// Pre-compiled glob pattern to avoid per-file allocations.
pub enum Glob {
    All,
    Exact(String),
    Parts { segments: Vec<String>, anchored_start: bool, anchored_end: bool },
}

impl Glob {
    pub fn compile(pattern: &str) -> Self {
        if pattern == "*" {
            return Self::All;
        }
        if !pattern.contains('*') {
            return Self::Exact(pattern.to_string());
        }
        let raw_parts: Vec<&str> = pattern.split('*').collect();
        let anchored_start = !raw_parts[0].is_empty();
        let anchored_end = !raw_parts[raw_parts.len() - 1].is_empty();
        let segments: Vec<String> = raw_parts.iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Self::Parts { segments, anchored_start, anchored_end }
    }

    pub fn matches(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Exact(p) => p == name,
            Self::Parts { segments, anchored_start, anchored_end } => {
                if segments.is_empty() {
                    return true;
                }
                let mut pos = 0;
                for (i, seg) in segments.iter().enumerate() {
                    if i == 0 && *anchored_start {
                        if !name.starts_with(seg.as_str()) {
                            return false;
                        }
                        pos = seg.len();
                    } else if i == segments.len() - 1 && *anchored_end {
                        if !name[pos..].ends_with(seg.as_str()) {
                            return false;
                        }
                        return true;
                    } else {
                        match name[pos..].find(seg.as_str()) {
                            Some(idx) => pos += idx + seg.len(),
                            None => return false,
                        }
                    }
                }
                true
            }
        }
    }
}
