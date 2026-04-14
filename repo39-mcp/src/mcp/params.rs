use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TreeParams {
    /// Target directory path (relative or absolute)
    pub path: String,

    /// Max depth (0=root only, default: 0). Use large N (e.g. 99) for full tree.
    #[serde(default)]
    pub depth: Option<usize>,

    /// Show filter: f=files d=dirs h=hidden c=count a=all (default: fd)
    #[serde(default)]
    pub show: Option<String>,

    /// Sort: n=name s=size m=modified c=created (default: n). s/m/c auto-enables corresponding info.
    #[serde(default)]
    pub order: Option<String>,

    /// Info to display: s=size m=modified c=created g=git t=tokens (combinable)
    #[serde(default)]
    pub info: Option<String>,

    /// Max entries per directory, 0=unlimited. Only applies below root.
    #[serde(default)]
    pub limit: Option<usize>,

    /// Grep files by name glob (e.g. "*.json", "Cargo.toml")
    #[serde(default)]
    pub grep: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdentifyParams {
    /// Target directory path (relative or absolute)
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapParams {
    /// Target directory path (relative or absolute)
    pub path: String,

    /// Max depth (default: 99)
    #[serde(default)]
    pub depth: Option<usize>,

    /// Max symbols per file (0=unlimited)
    #[serde(default)]
    pub limit: Option<usize>,

    /// Filter symbols by name glob
    #[serde(default)]
    pub grep: Option<String>,

    /// Show intra-file call graph (which functions call which)
    #[serde(default)]
    pub calls: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DepsParams {
    /// Target directory path (relative or absolute)
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChangesParams {
    /// Target directory path (relative or absolute)
    pub path: String,

    /// Branch diff spec (e.g. "main..HEAD", "main"). If set, shows branch diff instead of recent commits.
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummaryParams {
    /// Target directory path (relative or absolute)
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReviewParams {
    /// Target directory path (relative or absolute)
    pub path: String,

    /// Git ref to diff against (default: HEAD~1). Examples: HEAD~3, main, abc1234
    #[serde(default)]
    pub ref_spec: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Target directory path (relative or absolute)
    pub path: String,

    /// Search pattern (literal string or regex)
    pub pattern: String,

    /// Treat pattern as regex (default: false)
    #[serde(default)]
    pub is_regex: Option<bool>,

    /// Context lines around each match (default: 0)
    #[serde(default)]
    pub context: Option<usize>,

    /// Max matches to return (default: 50, 0=unlimited)
    #[serde(default)]
    pub max_results: Option<usize>,

    /// Filter files by name glob (e.g. "*.rs", "*.toml")
    #[serde(default)]
    pub file_glob: Option<String>,
}
