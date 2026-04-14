pub struct ShowFilter {
    pub files: bool,
    pub dirs: bool,
    pub hidden: bool,
    pub count: bool,
    pub max_depth: usize,
}

impl ShowFilter {
    pub fn parse(s: &str, max_depth: usize) -> Self {
        let count = s.contains('c');
        if s.contains('a') {
            return Self { files: true, dirs: true, hidden: true, count, max_depth };
        }
        Self {
            files: s.contains('f'),
            dirs: s.contains('d'),
            hidden: s.contains('h'),
            count,
            max_depth,
        }
    }
}

#[derive(Clone, Copy)]
pub enum SortOrder {
    Name,
    Size,
    Modified,
    Created,
}

impl SortOrder {
    pub fn parse(s: &str) -> Self {
        match s.chars().next().unwrap_or('n') {
            's' => Self::Size,
            'm' => Self::Modified,
            'c' => Self::Created,
            _ => Self::Name,
        }
    }
}

pub struct InfoFlags {
    pub size: bool,
    pub modified: bool,
    pub created: bool,
    pub git: bool,
}

impl InfoFlags {
    pub fn parse(s: &str, order: SortOrder) -> Self {
        let mut flags = Self {
            size: s.contains('s'),
            modified: s.contains('m'),
            created: s.contains('c'),
            git: s.contains('g'),
        };
        match order {
            SortOrder::Size => flags.size = true,
            SortOrder::Modified => flags.modified = true,
            SortOrder::Created => flags.created = true,
            SortOrder::Name => {}
        }
        flags
    }

    pub fn needs_metadata(&self) -> bool {
        self.size || self.modified || self.created
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum SizeUnit {
    K,
    M,
    G,
}

#[allow(dead_code)]
impl SizeUnit {
    pub fn parse(s: &str) -> Self {
        match s.chars().next().unwrap_or('K') {
            'M' | 'm' => Self::M,
            'G' | 'g' => Self::G,
            _ => Self::K,
        }
    }

    pub fn format(self, bytes: u64) -> String {
        match self {
            Self::K => {
                let kb = bytes as f64 / 1024.0;
                if kb < 10.0 { format!("{kb:.1}K") }
                else { format!("{}K", kb as u64) }
            }
            Self::M => {
                let mb = bytes as f64 / (1024.0 * 1024.0);
                if mb < 10.0 { format!("{mb:.2}M") }
                else { format!("{mb:.1}M") }
            }
            Self::G => {
                let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                format!("{gb:.2}G")
            }
        }
    }
}

