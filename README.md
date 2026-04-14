# repo39

Token-optimized repo explorer for AI agents.

Scans a directory and outputs a compact tree. Designed to minimize tokens so agents can understand repo structure fast and cheap.

## Install

```bash
cargo install --path .
```

## Quick Reference

```
repo39 <path> [-s fdhca] [-d N] [-n N] [-g glob] [-o nscm] [-i smcg] [-u KMG]
```

| Flag | What | Values | Default |
|------|------|--------|---------|
| `-s` | show | `f`=files `d`=dirs `h`=hidden `c`=count `a`=all | `fd` |
| `-d` | depth | `0`=root only, `1`..N, `99`=unlimited | `0` |
| `-n` | limit per subfolder | `0`=unlimited, `1`..N (root always unlimited) | `0` |
| `-g` | grep filename glob | `*.rs`, `pack*`, `Cargo.toml` | - |
| `-o` | sort | `n`=name `s`=size `m`=modified `c`=created | `n` |
| `-i` | info to display | `s`=size `m`=modified `c`=created `g`=git | - |
| `-u` | size unit | `K`=KB `M`=MB `G`=GB | `K` |

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
```

## Agent Workflow

### 1. First look at any repo

```bash
repo39 /project
```
```
CLAUDE.md
Cargo.lock
Cargo.toml
README.md
build.sh
docs/
src/
```

Root only. ~10 lines. Agent identifies: Rust project, has docs.

### 2. Explore structure

```bash
repo39 /project -d 1 -n 3
```
```
CLAUDE.md
Cargo.lock
Cargo.toml
README.md
build.sh
docs/
src/
 main.rs
```

One level deep, max 3 items per subfolder.

### 3. Find specific files

```bash
repo39 /project -g "*.json" -s a
```
```
.claude-plugin/
 plugin.json
.mcp.json
openclaw-plugin/
 openclaw.plugin.json
 package.json
```

Full depth search. Only matching files + ancestor dirs shown.

### 4. Check sizes and dates

```bash
repo39 /project -d 1 -i sm
```
```
CLAUDE.md 3.9K 2026-04-12
Cargo.lock 51K 2026-04-10
src/
 main.rs 22K 2026-04-10
```

### 5. Find largest files

```bash
repo39 /project -d 99 -o s -i s
```

Sort by size descending, show sizes. Each subfolder sorted independently.

### 6. Folder overview with file counts

```bash
repo39 /project -s fdc
```
```
CLAUDE.md
Cargo.toml
docs/ 0
openclaw-plugin/ 3
src/ 1
```

At depth limit, dirs show total file count in subtree.

### 7. Git dirty files

```bash
repo39 /project -d 99 -i g
```
```
src/
 *main.rs
README.md
```

Dirty files prefixed with `*`. Warns on non-git folders.

## Auto-skipped Directories

`.git`, `node_modules`, `target`, `__pycache__`, `.venv`, `venv`, `dist`, `.next`

## Cross-platform

Works on Linux, macOS, and Windows. Output always uses `/` path separators.
