use std::collections::HashSet;
use std::io::Write;

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
use crate::review::run_review;
use crate::search::run_search;
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
    #[tool(description = "List directory tree. Output: indented lines (1 space/depth), dirs end with /, files plain. Auto-skips .git, node_modules, target, etc.")]
    async fn repo39_tree(
        &self,
        Parameters(params): Parameters<TreeParams>,
    ) -> Result<String, String> {
        run_tree(params).map_err(|e| e.to_string())
    }

    #[tool(description = "Identify project type(s). Output: one line per match (max 5), format: `name score` (0.00-1.00, descending). Detects languages, frameworks, categories.")]
    async fn repo39_identify(
        &self,
        Parameters(params): Parameters<IdentifyParams>,
    ) -> Result<String, String> {
        capture(&params.path, run_identify).map_err(|e| e.to_string())
    }

    #[tool(description = "Extract code symbols from source files. Output: indented tree (dir/ file symbols). Symbol format: `prefix name:line` (e.g. `fn foo:10`, `class Bar:25`). With calls=true: `fn foo:10 -> bar, baz` showing intra-file call graph. 13 languages: rs py js ts go java kt rb php c/cpp swift ex dart sh.")]
    async fn repo39_map(
        &self,
        Parameters(params): Parameters<MapParams>,
    ) -> Result<String, String> {
        run_map_tool(params).map_err(|e| e.to_string())
    }

    #[tool(description = "List dependencies from manifest files. Output: `name version [dev]` per line. Supports Cargo.toml, package.json, pyproject.toml, go.mod, Gemfile, composer.json, requirements.txt. Workspace-aware.")]
    async fn repo39_deps(
        &self,
        Parameters(params): Parameters<DepsParams>,
    ) -> Result<String, String> {
        capture(&params.path, run_deps).map_err(|e| e.to_string())
    }

    #[tool(description = "Show file changes. Default: recent git log (last 100 commits). With branch param (e.g. 'main..HEAD'): branch diff. Output: `time_ago path [+ins] [-del] [new|del]`. Max 50 files.")]
    async fn repo39_changes(
        &self,
        Parameters(params): Parameters<ChangesParams>,
    ) -> Result<String, String> {
        if let Some(branch) = &params.branch {
            let root = canonicalize(std::path::Path::new(&params.path)).map_err(|e| e.to_string())?;
            let mut buf = Vec::with_capacity(4096);
            crate::changes::run_changes_branch(&root, branch, &mut buf).map_err(|e| e.to_string())?;
            Ok(String::from_utf8(buf).unwrap_or_default())
        } else {
            capture(&params.path, run_changes).map_err(|e| e.to_string())
        }
    }

    #[tool(description = "Search file contents. Output: `path:line content` per match. Skips binary/generated files, auto-skips .git, node_modules, target, etc. Groups separated by `--`.")]
    async fn repo39_search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<String, String> {
        run_search_tool(params).map_err(|e| e.to_string())
    }

    #[tool(description = "Show symbol-level changes between git refs. Output: `path` header then `+symbol:line` (added), `-symbol` (removed), `~symbol:line` (modified). Compares current vs ref_spec (default HEAD~1). Max 20 files.")]
    async fn repo39_review(
        &self,
        Parameters(params): Parameters<ReviewParams>,
    ) -> Result<String, String> {
        run_review_tool(params).map_err(|e| e.to_string())
    }

    #[tool(description = "One-shot repo orientation. Combines identify + deps + map (top 1 symbol/file) + changes into a single call. Use as first call when entering an unfamiliar repo.")]
    async fn repo39_summary(
        &self,
        Parameters(params): Parameters<SummaryParams>,
    ) -> Result<String, String> {
        run_summary_tool(params).map_err(|e| e.to_string())
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
            "Token-optimized repository explorer for AI agents. \
             Tools: repo39_tree (directory structure), repo39_identify (project type), \
             repo39_map (code symbols), repo39_deps (dependencies), repo39_changes (git changes), \
             repo39_search (content search), repo39_review (symbol-level diff), \
             repo39_summary (one-shot repo orientation).",
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
    let calls = params.calls.unwrap_or(false);
    let mut buf = Vec::with_capacity(4096);
    run_map(&root, depth, limit, params.grep.as_deref(), calls, &mut buf)?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}

fn run_summary_tool(params: SummaryParams) -> std::io::Result<String> {
    let root = canonicalize(std::path::Path::new(&params.path))?;
    let mut buf = Vec::with_capacity(8192);
    writeln!(buf, "[identify]")?;
    run_identify(&root, &mut buf)?;
    writeln!(buf, "\n[deps]")?;
    run_deps(&root, &mut buf)?;
    writeln!(buf, "\n[map]")?;
    run_map(&root, 99, 1, None, false, &mut buf)?;
    writeln!(buf, "\n[changes]")?;
    run_changes(&root, &mut buf)?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}

fn run_review_tool(params: ReviewParams) -> std::io::Result<String> {
    let root = canonicalize(std::path::Path::new(&params.path))?;
    let mut buf = Vec::with_capacity(4096);
    run_review(&root, params.ref_spec.as_deref(), &mut buf)?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}

fn run_search_tool(params: SearchParams) -> std::io::Result<String> {
    let root = canonicalize(std::path::Path::new(&params.path))?;
    let is_regex = params.is_regex.unwrap_or(false);
    let context = params.context.unwrap_or(0);
    let max_results = params.max_results.unwrap_or(50);
    let mut buf = Vec::with_capacity(4096);
    run_search(&root, &params.pattern, is_regex, context, max_results, params.file_glob.as_deref(), &mut buf)?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}
