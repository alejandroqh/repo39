use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TreeParams {
    /// Target directory path (relative or absolute)
    pub path: String,

    /// Max depth (0=root only, default: 0)
    #[serde(default)]
    pub depth: Option<usize>,

    /// Show filter: f=files d=dirs h=hidden c=count a=all (default: fd)
    #[serde(default)]
    pub show: Option<String>,

    /// Sort: n=name s=size m=modified c=created (default: n)
    #[serde(default)]
    pub order: Option<String>,

    /// Info to display: s=size m=modified c=created g=git (combinable)
    #[serde(default)]
    pub info: Option<String>,

    /// Max files per directory (0=unlimited)
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
}
