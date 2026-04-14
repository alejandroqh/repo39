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
              [--identify] [--map] [--deps] [--changes]
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
| `--identify` | detect project type(s) with confidence | - | - |
| `--map` | extract code symbols (fn, struct, class) | uses `-d`, `-n`, `-g` | - |
| `--deps` | list dependencies from manifest files | - | - |
| `--changes` | compact git log (recent file changes) | - | - |

Standalone flags (`--identify`, `--map`, `--deps`, `--changes`) can be combined. When multiple are used, output is sectioned with `[label]` headers.

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

### 1. Identify the project

```bash
repo39 /project --identify
```
```
rust 0.85
markdown 0.39
docs 0.12
```

52 categories: languages, frameworks, non-code (images, data, docs, devops, etc). Top 5 matches with confidence 0.00-1.00.

### 2. Read dependencies

```bash
repo39 /project --deps
```
```
clap 4
regex 1
tempfile 3 dev
```

Parses: Cargo.toml, package.json, pyproject.toml, requirements.txt, go.mod, Gemfile, composer.json. Shows `dev` suffix for dev dependencies.

### 3. Get the code map

```bash
repo39 /project --map -d 99
```
```
src/main.rs
 fn main
src/walk.rs
 struct WalkCtx
 fn walk
 fn grep_walk
```

12 languages: Rust, Python, JS, TS, Go, Java, Kotlin, Ruby, PHP, C/C++, Swift, Elixir, Shell.

Limit symbols per file:
```bash
repo39 /project --map -d 99 -n 3
```
```
src/config.rs
 struct Cli
 struct ShowFilter
 impl ShowFilter
 ...+9
```

Search for a specific symbol:
```bash
repo39 /project --map -d 99 -g "login*"
```
```
src/auth.rs
 fn login
 fn login_handler
```

### 4. Check recent activity

```bash
repo39 /project --changes
```
```
2h src/main.rs +8 -3
5d src/walk.rs +12 -8
1w README.md +95 new
```

Time-relative (`3m`, `2h`, `1d`, `2w`, `3M`, `1y`). Shows insertions/deletions, `new`/`del` markers.

### 5. Full picture in one command

```bash
repo39 /project --identify --deps --map -d 1
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
 fn main
src/walk.rs
 struct WalkCtx
 fn walk
```

### 6. Explore structure

```bash
repo39 /project -d 1 -n 3
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

### 7. Find specific files

```bash
repo39 /project -g "*.json" -s a
```
```
.mcp.json
config/
 settings.json
```

Full depth search. Only matching files + ancestor dirs shown.

### 8. Check sizes and dates

```bash
repo39 /project -d 1 -i sm
```
```
Cargo.lock 51K 2026-04-10
src/
 main.rs 22K 2026-04-10
```

### 9. Git dirty files

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

Note: `--identify` does NOT skip these — their presence is a detection signal.

## Cross-platform

Works on Linux, macOS, and Windows. Output always uses `/` path separators.
