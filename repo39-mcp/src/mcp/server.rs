use std::collections::HashSet;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{ServerHandler, tool, tool_handler, tool_router};

use super::params::*;
use crate::config::{InfoFlags, ShowFilter, SizeUnit, SortOrder};
use crate::changes::run_changes;
use crate::deps::run_deps;
use crate::git::load_git_dirty;
use crate::glob::Glob;
use crate::identify::run_identify;
use crate::map::run_map;
use crate::util::canonicalize;
use crate::walk::{grep_walk, walk, WalkCtx};

#[allow(dead_code)]
pub struct McpServer {
    tool_router: ToolRouter<Self>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl McpServer {
    #[tool(description = "List directory tree structure. Token-optimized output with configurable depth, filtering, sorting, and file info.")]
    async fn repo39_tree(
        &self,
        Parameters(params): Parameters<TreeParams>,
    ) -> Result<String, String> {
        run_tree(params).map_err(|e| e.to_string())
    }

    #[tool(description = "Identify project type(s) with confidence scores. Detects languages, frameworks, and project categories (52 types).")]
    async fn repo39_identify(
        &self,
        Parameters(params): Parameters<IdentifyParams>,
    ) -> Result<String, String> {
        capture(&params.path, run_identify).map_err(|e| e.to_string())
    }

    #[tool(description = "Extract code symbols (functions, structs, classes) from source files. Supports 12+ languages.")]
    async fn repo39_map(
        &self,
        Parameters(params): Parameters<MapParams>,
    ) -> Result<String, String> {
        run_map_tool(params).map_err(|e| e.to_string())
    }

    #[tool(description = "List project dependencies from manifest files (Cargo.toml, package.json, pyproject.toml, go.mod, Gemfile, composer.json, requirements.txt).")]
    async fn repo39_deps(
        &self,
        Parameters(params): Parameters<DepsParams>,
    ) -> Result<String, String> {
        capture(&params.path, run_deps).map_err(|e| e.to_string())
    }

    #[tool(description = "Show recent file changes from git history. Compact format: time_ago path +insertions -deletions [new|del].")]
    async fn repo39_changes(
        &self,
        Parameters(params): Parameters<ChangesParams>,
    ) -> Result<String, String> {
        capture(&params.path, run_changes).map_err(|e| e.to_string())
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::new("repo39-mcp", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "repo39-mcp is a token-optimized repository explorer for AI agents. \
             Use repo39_tree to list directory structure, repo39_identify to detect project type, \
             repo39_map to extract code symbols, repo39_deps to list dependencies, \
             and repo39_changes to show recent git changes. \
             All output is optimized for minimal token usage.",
        )
    }
}

fn capture(
    path: &str,
    f: fn(&std::path::Path, &mut Vec<u8>) -> std::io::Result<()>,
) -> std::io::Result<String> {
    let root = canonicalize(std::path::Path::new(path))?;
    let mut buf = Vec::with_capacity(4096);
    f(&root, &mut buf)?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}

fn run_tree(params: TreeParams) -> std::io::Result<String> {
    let mut dir_buf = canonicalize(std::path::Path::new(&params.path))?;
    let depth = params.depth.unwrap_or(0);
    let show = params.show.as_deref().unwrap_or("fd");
    let order_str = params.order.as_deref().unwrap_or("n");
    let info_str = params.info.as_deref().unwrap_or("");
    let limit = params.limit.unwrap_or(0);

    let filter = ShowFilter::parse(show, depth);
    let order = SortOrder::parse(order_str);
    let info = InfoFlags::parse(info_str, order);

    let dirty_files = if info.git {
        load_git_dirty(&dir_buf, true)
    } else {
        HashSet::new()
    };

    let root = dir_buf.clone();
    let ctx = WalkCtx {
        root: &root,
        filter,
        order,
        info,
        unit: SizeUnit::K,
        limit,
        dirty_files,
    };

    let mut buf = Vec::with_capacity(4096);
    if let Some(ref pattern) = params.grep {
        let glob = Glob::compile(pattern);
        grep_walk(&mut dir_buf, &ctx, &glob, 0, &mut buf)?;
    } else {
        walk(&mut dir_buf, &ctx, 0, &mut buf)?;
    }

    Ok(String::from_utf8(buf).unwrap_or_default())
}

fn run_map_tool(params: MapParams) -> std::io::Result<String> {
    let root = canonicalize(std::path::Path::new(&params.path))?;
    let depth = params.depth.unwrap_or(99);
    let limit = params.limit.unwrap_or(0);
    let mut buf = Vec::with_capacity(4096);
    run_map(&root, depth, limit, params.grep.as_deref(), &mut buf)?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}
