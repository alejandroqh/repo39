# repo39

Token-optimized repo explorer for AI agents.

Scans a directory and outputs a compact tree. Designed to minimize tokens so agents can understand repo structure fast and cheap.

## Install

```bash
cargo install repo39-cli repo39-mcp   # both
cargo install repo39-cli              # CLI only
cargo install repo39-mcp              # MCP server only
```

## Quick Reference

```
repo39-cli <path> [flags] [commands]

Flags: -s fdhca  -d N  -n N  -g glob  -o nscm  -i smcgt  -u KMG

Commands:
  --identify    --map [--calls]    --deps
  --changes     --summary          --review [ref]
  --search <pat> [--regex] [--context N]
                 [--file-filter glob] [--max-results N]
```

| Flag | What | Values | Default |
|------|------|--------|---------|
| `-s` | show | `f`=files `d`=dirs `h`=hidden `c`=count `a`=all | `fd` |
| `-d` | depth | `0`=root only, `1`..N, `99`=unlimited | `0` |
| `-n` | limit per subfolder | `0`=unlimited, `1`..N (root always unlimited) | `0` |
| `-g` | grep filename glob | `*.rs`, `pack*`, `Cargo.toml` | - |
| `-o` | sort | `n`=name `s`=size `m`=modified `c`=created | `n` |
| `-i` | info to display | `s`=size `m`=modified `c`=created `g`=git `t`=tokens | - |
| `-u` | size unit | `K`=KB `M`=MB `G`=GB | `K` |
| `--identify` | detect project type(s) with confidence | - | - |
| `--map` | extract code symbols (fn, struct, class) | uses `-d`, `-n`, `-g` | - |
| `--calls` | show intra-file call graph (with `--map`) | - | - |
| `--deps` | list dependencies from manifest files | - | - |
| `--changes` | compact git log (recent file changes) | - | - |
| `--summary` | one-shot orientation (identify+deps+map+changes) | - | - |
| `--search` | search file contents | literal or regex pattern | - |
| `--regex` | treat `--search` pattern as regex | - | off |
| `--context` | context lines around search matches | `0`..N | `0` |
| `--file-filter` | file glob for search | `*.rs`, `*.toml` | - |
| `--max-results` | max search matches | `0`=unlimited, N | `50` |
| `--review` | symbol-level diff vs git ref | git ref (e.g. `main`, `HEAD~3`) | `HEAD~1` |

Standalone flags (`--identify`, `--map`, `--deps`, `--changes`, `--summary`, `--search`, `--review`) can be combined. When multiple are used, output is sectioned with `[label]` headers.

## Output Format

```
name/       directory (trailing /)
name        file (no suffix)
 name       depth 1 (1 space indent per level)
  name      depth 2
...+N       truncated (N hidden entries)
name/ 3     dir with file count (-s c at depth limit)
*name       git dirty file (-i g)
name 1.4K   file with size (-i s)
name 2026-04-10   file with date (-i m or -i c)
name 450     file with token count (-i t)
10:+fn foo   public symbol with line number (--map)
20:fn bar -> baz, qux   call graph (--map --calls)
```

## Agent Workflow

Two interfaces, same output. Use the CLI from a shell or the MCP server from any MCP-compatible agent.

### 1. Identify the project

```bash
repo39-cli /project --identify
```
```python
repo39_identify { "path": "/project" }
```
```
rust 0.85
markdown 0.39
docs 0.12
```

71 categories: 22 languages, 37 frameworks, 12 non-code (images, data, docs, devops, etc). Top 5 matches with confidence 0.00-1.00. Framework detection uses dependency analysis (e.g. detects React, Django, Spring from manifests).

### 2. Read dependencies

```bash
repo39-cli /project --deps
```
```python
repo39_deps { "path": "/project" }
```
```
clap 4
regex 1
tempfile 3 dev
```

Parses: Cargo.toml, package.json, pyproject.toml, requirements.txt, go.mod, Gemfile, composer.json. Shows `dev` suffix for dev dependencies.

### 3. Get the code map

13 languages: Rust, Python, JS, TS, Go, Java/Kotlin, Ruby, PHP, C/C++, Swift, Elixir, Dart, Shell.

Symbols include line numbers and visibility (`+` = public):
```bash
repo39-cli /project --map -d 99
```
```python
repo39_map { "path": "/project", "depth": 99 }
```
```
src/main.rs
 1:fn main
src/walk.rs
 5:+struct WalkCtx
 12:+fn walk
 30:fn grep_walk
```

Show call graph:
```bash
repo39-cli /project --map --calls -d 99
```
```python
repo39_map { "path": "/project", "depth": 99, "calls": true }
```
```
src/walk.rs
 12:+fn walk -> grep_walk
 30:fn grep_walk
```

Limit symbols per file:
```bash
repo39-cli /project --map -d 99 -n 3
```
```python
repo39_map { "path": "/project", "depth": 99, "limit": 3 }
```
```
src/config.rs
 5:struct Cli
 12:struct ShowFilter
 20:impl ShowFilter
 ...+9
```

Search for a specific symbol:
```bash
repo39-cli /project --map -d 99 -g "login*"
```
```python
repo39_map { "path": "/project", "depth": 99, "grep": "login*" }
```
```
src/auth.rs
 8:fn login
 15:fn login_handler
```

### 4. Check recent activity

```bash
repo39-cli /project --changes
```
```python
repo39_changes { "path": "/project" }
```
```
2h src/main.rs +8 -3
5d src/walk.rs +12 -8
1w README.md +95 new
```

Time-relative (`3m`, `2h`, `1d`, `2w`, `3M`, `1y`). Shows insertions/deletions, `new`/`del` markers.

Branch diff:
```bash
repo39-cli /project --changes main..HEAD
```
```python
repo39_changes { "path": "/project", "branch": "main..HEAD" }
```

### 5. Search file contents

```bash
repo39-cli /project --search "TODO"
```
```python
repo39_search { "path": "/project", "pattern": "TODO" }
```
```
src/main.rs:42 // TODO: handle error
src/walk.rs:18 // TODO: optimize
```

With regex and context:
```bash
repo39-cli /project --search "fn\s+test_" --regex --context 1 --file-filter "*.rs"
```
```python
repo39_search {
  "path": "/project",
  "pattern": "fn\\s+test_",
  "is_regex": true,
  "context": 1,
  "file_glob": "*.rs"
}
```
```
src/walk.rs
17-
18:fn test_walk() {
19-    let ctx = WalkCtx::new();
```

Max 50 results by default. Use `--max-results 0` / `"max_results": 0` for unlimited.

### 6. Review symbol-level changes

```bash
repo39-cli /project --review
```
```python
repo39_review { "path": "/project" }
```
```
src/main.rs
 +45:fn new_feature
 ~1:fn main
src/old.rs
 -fn deprecated
```

Compares against `HEAD~1` by default. Specify a ref:
```bash
repo39-cli /project --review main
```
```python
repo39_review { "path": "/project", "ref_spec": "main" }
```

Symbols: `+` = added, `-` = removed, `~` = modified. Max 20 changed files.

### 7. Full picture in one command

```bash
repo39-cli /project --summary
```
```python
repo39_summary { "path": "/project" }
```
```
[identify]
rust 0.85
markdown 0.39

[deps]
clap 4
regex 1

[map]
src/main.rs
 1:fn main
src/walk.rs
 5:+struct WalkCtx
 ...+3

[changes]
2h src/main.rs +8 -3
5d src/walk.rs +12 -8
```

Combines identify + deps + map (depth 99, 1 symbol/file) + changes. Equivalent to `--identify --deps --map -d 99 -n 1 --changes`.

### 8. Explore structure

```bash
repo39-cli /project -d 1 -n 3
```
```python
repo39_tree { "path": "/project", "depth": 1, "limit": 3 }
```
```
Cargo.toml
README.md
src/
 main.rs
 walk.rs
 config.rs
 ...+4
```

One level deep, max 3 items per subfolder.

### 9. Find specific files

```bash
repo39-cli /project -g "*.json" -s a
```
```python
repo39_tree { "path": "/project", "grep": "*.json", "show": "a" }
```
```
.mcp.json
config/
 settings.json
```

Full depth search. Only matching files + ancestor dirs shown.

### 10. Check sizes and dates

```bash
repo39-cli /project -d 1 -i sm
```
```python
repo39_tree { "path": "/project", "depth": 1, "info": "sm" }
```
```
Cargo.lock 51K 2026-04-10
src/
 main.rs 22K 2026-04-10
```

### 11. Git dirty files

```bash
repo39-cli /project -d 99 -i g
```
```python
repo39_tree { "path": "/project", "depth": 99, "info": "g" }
```
```
src/
 *main.rs
README.md
```

Dirty files prefixed with `*`. Warns on non-git folders.

## Auto-skipped Directories

`.git`, `node_modules`, `target`, `__pycache__`, `.venv`, `venv`, `dist`, `.next`

Note: `--identify` does NOT skip these — their presence is a detection signal.

## MCP Server

`repo39-mcp` exposes the same capabilities as an MCP server over stdio. Configure via `.mcp.json`:

```json
{
  "mcpServers": {
    "repo39": {
      "command": "repo39-mcp"
    }
  }
}
```

### Tools

| Tool | Description | Extra params vs CLI |
|------|-------------|---------------------|
| `repo39_tree` | directory tree listing | - |
| `repo39_identify` | project type detection | - |
| `repo39_map` | code symbol extraction | `calls` (bool) |
| `repo39_deps` | dependency parsing | - |
| `repo39_changes` | git change log | `branch` (e.g. `main..HEAD`) |
| `repo39_search` | content search | `file_glob`, `is_regex`, `context`, `max_results` |
| `repo39_review` | symbol-level diff | `ref_spec` |
| `repo39_summary` | one-shot repo orientation (identify + deps + map + changes) | - |

All tools take a required `path` parameter. Tree/map tools accept `depth`, `limit`, `grep` as in the CLI.

## Benchmark: repo39 vs standard tools

`repo39-cli --summary` vs the equivalent shell commands (`ls`, `find`, `cat`, `grep -rn`, `git log --stat`) to get the same repo orientation.

### Results

| | Calls | Tokens | Bytes | Savings |
|---|---|---|---|---|
| **[express](https://github.com/expressjs/express)** (~240 files) | | | | |
| standard | 5 | 1,727 | 24,926 | — |
| repo39 | 1 | 479 | 4,504 | **72% tokens, 81% bytes** |
| **[fastapi](https://github.com/tiangolo/fastapi)** (~3k files) | | | | |
| standard | 5 | 26,640 | 643,161 | — |
| repo39 | 1 | 3,281 | 49,545 | **87% tokens, 92% bytes** |

### Where the savings come from

fastapi standard breakdown:
```
                        Calls   Lines Words(≈tok)      Bytes
identify (ls + find)        2      28           47        301
deps (cat manifests)        1     340          928      10497
map (grep definitions)      1    4381        13345     447513
changes (git log --stat)    1    2986        12320     184850
```

The biggest win is symbol extraction: `grep -rn` outputs full matching lines while repo39 outputs compact `line:name` format — 99% byte reduction on map alone.

### Reproduce

`benchmark.sh` is included in the repo. Run it against any project:
```bash
./benchmark.sh /path/to/repo
```

## Cross-platform

Works on Linux, macOS, and Windows. Output always uses `/` path separators.
