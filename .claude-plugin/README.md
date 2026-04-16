# repo39 — Claude Plugin

Token-optimized repository explorer for AI agents.

## Install

```bash
cargo install repo39-mcp
```

## MCP Tools

- `repo39_tree` — List directory tree structure
- `repo39_identify` — Identify project type(s) with confidence scores
- `repo39_map` — Extract code symbols (functions, structs, classes)
- `repo39_deps` — List project dependencies from manifest files
- `repo39_changes` — Show recent file changes from git history
- `repo39_search` — Search file contents (literal or regex)
- `repo39_review` — Symbol-level diff between git refs
- `repo39_summary` — One-shot repo orientation (identify + deps + map + changes)
