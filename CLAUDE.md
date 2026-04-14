# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                          # build all (cli + mcp)
cargo build -p repo39-cli            # build CLI only
cargo build -p repo39-mcp            # build MCP server only
cargo build --release                # optimized release build
cargo run -- <args>                  # run CLI (default member)
cargo test                           # run all tests
cargo test -p repo39-cli             # run CLI tests
```

## Architecture

Cargo workspace with two standalone binary crates:

### repo39-cli
Rust CLI using `clap` (v4, derive API). Single-binary, edition 2024.

Takes a folder path, outputs a token-compact tree listing. Auto-skips noisy dirs (.git, node_modules, target, etc).

CLI flags:
- `-s <chars>` — show filter: `f`=files `d`=dirs `h`=hidden `c`=count `a`=all (default: `fd`)
- `-d <N>` — max depth: `0`=root only (default), `1`=root+one level, large N=unlimited
- `-o <char>` — sort: `n`=name `s`=size `m`=modified `c`=created
- `-i <chars>` — info: `s`=size `m`=modified `c`=created `g`=git `t`=tokens
- `-n <N>` — max files per directory
- `-g <glob>` — grep files by name glob
- `--identify` — detect project type(s) with confidence scores
- `--map` — extract code symbols (functions, structs, classes)
- `--deps` — list dependencies from manifest files
- `--changes` — show recent git changes
- `--summary` — one-shot orientation (identify + deps + map + changes)
- `--search <pattern>` — search file contents (with `--regex`, `--context N`, `--file-filter`, `--max-results`)
- `--review [ref]` — symbol-level diff vs git ref (default: HEAD~1)

### repo39-mcp
MCP server using `rmcp` (TurboMCP). Runs over stdio transport.

Exposes 8 tools:
- `repo39_tree` — directory tree listing (info flag `t` for token estimates)
- `repo39_identify` — project type detection (with category: lang/framework/non-code, dep-based framework detection)
- `repo39_map` — code symbol extraction (with line numbers and visibility markers)
- `repo39_deps` — dependency parsing (workspace-aware with shared/mismatch analysis)
- `repo39_changes` — git change log (with optional `branch` param for branch diffs like `main..HEAD`)
- `repo39_search` — token-compact content search (skips binary, supports regex)
- `repo39_review` — symbol-level diff between git refs
- `repo39_summary` — one-shot repo orientation (identify + deps + map + changes)

Configured as local MCP server via `.mcp.json`. Use the repo39 MCP tools to test the project.

## Design Principles

This is a support tool for AI agents. Primary optimization target: **minimize token usage**.

- All output must be as compact as possible — agents pay per token
- Prefer terse, structured output over human-friendly prose
- Avoid decorative output: no banners, no progress bars, no color codes
- Error messages: short, actionable, single-line
- When in doubt between readable and compact, choose compact

Release profile: strip + LTO + single codegen unit.
