# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build              # dev build
cargo build --release    # optimized release build
cargo run                # run the CLI
cargo test               # run all tests
cargo test test_name     # run a single test
```

## Architecture

Rust CLI using `clap` (v4, derive API). Single-binary, edition 2024.

- `src/main.rs` — entry point, CLI parsing, recursive directory walker

Takes a folder path (relative or absolute), outputs a token-compact tree listing. Auto-skips noisy dirs (.git, node_modules, target, etc).

Output format: indented tree (1 space per depth level). `name/` = dir, `name` = file. Sorted alphabetically per level.

CLI flags:
- `-s <chars>` — show filter: `f`=files `d`=dirs `h`=hidden `c`=count `a`=all (default: `fd`)
- `-d <N>` — max depth: `0`=root only (default), `1`=root+one level, large N=unlimited
- `-s c` appends file count to truncated dirs: `src/ 3`

Release profile: strip + LTO + single codegen unit.

## Design Principles

This is a support tool for AI agents. Primary optimization target: **minimize token usage**.

- All output must be as compact as possible — agents pay per token for every byte of stdout/stderr
- Prefer terse, structured output (key=value, single-line records) over human-friendly prose
- Avoid decorative output: no banners, no progress bars, no color codes unless explicitly requested
- Error messages: short, actionable, single-line
- When in doubt between readable and compact, choose compact
