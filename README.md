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
+fn foo:10   public symbol with line number (--map)
fn bar:20 -> baz, qux   call graph (--map --calls)
```

## Agent Workflow

### 1. Identify the project

```bash
repo39-cli /project --identify
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
```
src/main.rs
 fn main:1
src/walk.rs
 +struct WalkCtx:5
 +fn walk:12
 fn grep_walk:30
```

Show call graph:
```bash
repo39-cli /project --map --calls -d 99
```
```
src/walk.rs
 +fn walk:12 -> grep_walk
 fn grep_walk:30
```

Limit symbols per file:
```bash
repo39-cli /project --map -d 99 -n 3
```
```
src/config.rs
 struct Cli:5
 struct ShowFilter:12
 impl ShowFilter:20
 ...+9
```

Search for a specific symbol:
```bash
repo39-cli /project --map -d 99 -g "login*"
```
```
src/auth.rs
 fn login:8
 fn login_handler:15
```

### 4. Check recent activity

```bash
repo39-cli /project --changes
```
```
2h src/main.rs +8 -3
5d src/walk.rs +12 -8
1w README.md +95 new
```

Time-relative (`3m`, `2h`, `1d`, `2w`, `3M`, `1y`). Shows insertions/deletions, `new`/`del` markers.

### 5. Search file contents

```bash
repo39-cli /project --search "TODO"
```
```
src/main.rs:42 // TODO: handle error
src/walk.rs:18 // TODO: optimize
```

With regex and context:
```bash
repo39-cli /project --search "fn\s+test_" --regex --context 1 --file-filter "*.rs"
```
```
src/walk.rs
17-
18:fn test_walk() {
19-    let ctx = WalkCtx::new();
```

Max 50 results by default. Use `--max-results 0` for unlimited.

### 6. Review symbol-level changes

```bash
repo39-cli /project --review
```
```
src/main.rs
 +fn new_feature:45
 ~fn main:1
src/old.rs
 -fn deprecated
```

Compares against `HEAD~1` by default. Specify a ref:
```bash
repo39-cli /project --review main
```

Symbols: `+` = added, `-` = removed, `~` = modified. Max 20 changed files.

### 7. Full picture in one command

```bash
repo39-cli /project --summary
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
 fn main:1
src/walk.rs
 +struct WalkCtx:5
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
```
Cargo.lock 51K 2026-04-10
src/
 main.rs 22K 2026-04-10
```

### 11. Git dirty files

```bash
repo39-cli /project -d 99 -i g
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

| | Small Rust workspace (~30 files) | Large Flutter app (~500 files) |
|---|---|---|
| **repo39 calls** | 1 | 1 |
| **standard calls** | 8 | 4 |
| **repo39 tokens** | 365 | 999 |
| **standard tokens** | 2,749 | 7,685 |
| **token savings** | **86%** | **87%** |
| **byte savings** | **92%** | **95%** |

### Where the savings come from

```
standard breakdown      Calls   Lines Words(≈tok)      Bytes
identify (ls + find)        2      17           27        187
deps (cat manifests)        4      58          186       1272
map (grep definitions)      1     309         1815      34594
changes (git log --stat)    1     168          721       8016
```

The biggest win is symbol extraction: `grep -rn` outputs full matching lines while repo39 outputs compact `fn name:line` format — ~95% byte reduction on map alone.

### Reproduce

```bash
./benchmark.sh /path/to/repo
```

## Cross-platform

Works on Linux, macOS, and Windows. Output always uses `/` path separators.
