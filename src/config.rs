use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "repo39",
    version,
    about = "Token-optimized repo explorer for AI agents"
)]
pub struct Cli {
    /// Target directory (relative or absolute)
    pub path: PathBuf,

    /// Show filter: f=files d=dirs h=hidden c=count a=all [default: fd]
    #[arg(short, long, default_value = "fd")]
    pub show: String,

    /// Max depth (0=root only, default)
    #[arg(short, long, default_value = "0")]
    pub depth: usize,

    /// Grep files by name glob (e.g. "*.json", "pack*", "Cargo.toml")
    #[arg(short, long)]
    pub grep: Option<String>,

    /// Sort: n=name(default) s=size m=modified c=created
    #[arg(short, long, default_value = "n")]
    pub order: String,

    /// Info to display: s=size m=modified c=created g=git (combinable)
    #[arg(short, long, default_value = "")]
    pub info: String,

    /// Size unit: K=KB M=MB G=GB (default: K)
    #[arg(short, long, default_value = "K")]
    pub unit: String,

    /// Max files per directory (0=unlimited, default)
    #[arg(short = 'n', long = "limit", default_value = "0")]
    pub limit: usize,

    /// Identify project type(s) with confidence scores
    #[arg(long)]
    pub identify: bool,
}

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
pub enum SizeUnit {
    K,
    M,
    G,
}

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

