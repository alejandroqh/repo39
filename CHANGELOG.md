# Changelog

## 1.0.0

Initial public release.

### repo39-cli

- Directory tree listing with depth, filtering, sorting, and info flags
- Code symbol extraction (`--map`) for 13 languages: Rust, Python, JS, TS, Go, Java/Kotlin, Ruby, PHP, C/C++, Swift, Elixir, Dart, Shell
- Intra-file call graph (`--map --calls`)
- Project type detection (`--identify`) — 71 categories (22 languages, 37 frameworks, 12 non-code)
- Dependency parsing (`--deps`) — Cargo.toml, package.json, pyproject.toml, requirements.txt, go.mod, Gemfile, composer.json
- Git change log (`--changes`) with time-relative timestamps
- Content search (`--search`) with regex, context lines, and file filtering
- Symbol-level diff (`--review`) between git refs
- One-shot orientation (`--summary`) combining identify + deps + map + changes
- Git dirty file markers (`-i g`)
- Token count estimation (`-i t`)
- Auto-skip noisy directories (.git, node_modules, target, etc.)

### repo39-mcp

- MCP server over stdio exposing 8 tools: `repo39_tree`, `repo39_identify`, `repo39_map`, `repo39_deps`, `repo39_changes`, `repo39_search`, `repo39_review`, `repo39_summary`
- Same output format as CLI — agents get identical results from either interface

### Design

- Token-optimized output: 72-87% fewer tokens vs standard shell tools
- Compact `line:symbol` format for code maps
- Cross-platform: Linux, macOS, Windows
- Single static binaries with LTO and symbol stripping
