#!/usr/bin/env bash
set -euo pipefail

# repo39 benchmark: compares repo39-cli --summary vs standard shell tools
# Usage: ./benchmark.sh <target-directory>

TARGET="${1:-.}"
TARGET=$(cd "$TARGET" && pwd)

# --- locate repo39-cli binary ---
if command -v repo39-cli &>/dev/null; then
    R39=repo39-cli
elif [ -x "./target/release/repo39-cli" ]; then
    R39="./target/release/repo39-cli"
elif [ -x "./target/debug/repo39-cli" ]; then
    R39="./target/debug/repo39-cli"
else
    echo "error: repo39-cli not found (PATH, target/release, target/debug)"
    exit 1
fi

# --- measurement helpers ---
measure() {
    local input="$1"
    printf '%s' "$input" | wc -lwc | awk '{print $1, $2, $3}'
}

SKIP_DIRS=(-not -path '*/.git/*' -not -path '*/target/*' -not -path '*/node_modules/*' -not -path '*/__pycache__/*' -not -path '*/.venv/*' -not -path '*/venv/*' -not -path '*/dist/*' -not -path '*/.next/*')

# ============================================================
# Phase 1: repo39
# ============================================================
r39_out=$("$R39" "$TARGET" --summary 2>/dev/null || true)
r39_m=$(measure "$r39_out")
r39_calls=1

# ============================================================
# Phase 2: standard method
# ============================================================
std_calls=0
std_total_lines=0
std_total_words=0
std_total_bytes=0

# per-section accumulators (bash 3 compatible)
id_calls=0; id_lines=0; id_words=0; id_bytes=0
dep_calls=0; dep_lines=0; dep_words=0; dep_bytes=0
map_calls=0; map_lines=0; map_words=0; map_bytes=0
chg_calls=0; chg_lines=0; chg_words=0; chg_bytes=0

add_section() {
    local section="$1" l="$2" w="$3" b="$4"
    case "$section" in
        identify) id_calls=$((id_calls+1)); id_lines=$((id_lines+l)); id_words=$((id_words+w)); id_bytes=$((id_bytes+b)) ;;
        deps)     dep_calls=$((dep_calls+1)); dep_lines=$((dep_lines+l)); dep_words=$((dep_words+w)); dep_bytes=$((dep_bytes+b)) ;;
        map)      map_calls=$((map_calls+1)); map_lines=$((map_lines+l)); map_words=$((map_words+w)); map_bytes=$((map_bytes+b)) ;;
        changes)  chg_calls=$((chg_calls+1)); chg_lines=$((chg_lines+l)); chg_words=$((chg_words+w)); chg_bytes=$((chg_bytes+b)) ;;
    esac
    std_calls=$((std_calls+1))
    std_total_lines=$((std_total_lines+l))
    std_total_words=$((std_total_words+w))
    std_total_bytes=$((std_total_bytes+b))
}

run_std() {
    local section="$1"; shift
    local out
    out=$("$@" 2>/dev/null || true)
    local m l w b
    m=$(measure "$out")
    read -r l w b <<< "$m"
    add_section "$section" "$l" "$w" "$b"
}

# --- identify: ls + find extensions ---
run_std identify ls -1 "$TARGET"
ext_out=$(find "$TARGET" -type f "${SKIP_DIRS[@]}" -name '*.*' 2>/dev/null \
    | sed 's|.*/||; s/.*\.//' | sort | uniq -c | sort -rn | head -20 || true)
m=$(measure "$ext_out"); read -r l w b <<< "$m"
add_section identify "$l" "$w" "$b"

# --- deps: cat each manifest that exists ---
MANIFESTS=(Cargo.toml package.json pyproject.toml requirements.txt go.mod Gemfile composer.json)
for mf in "${MANIFESTS[@]}"; do
    if [ -f "$TARGET/$mf" ]; then
        run_std deps cat "$TARGET/$mf"
    fi
done
# workspace members (Cargo)
if [ -f "$TARGET/Cargo.toml" ]; then
    members=$(sed -n '/members/,/]/p' "$TARGET/Cargo.toml" 2>/dev/null \
        | grep -o '"[^"]*"' | tr -d '"' || true)
    for member in $members; do
        if [ -f "$TARGET/$member/Cargo.toml" ]; then
            run_std deps cat "$TARGET/$member/Cargo.toml"
        fi
    done
fi

# --- map: grep for symbol definitions ---
run_std map grep -rn \
    --include='*.rs' --include='*.py' --include='*.js' --include='*.ts' --include='*.tsx' \
    --include='*.go' --include='*.java' --include='*.kt' --include='*.rb' --include='*.php' \
    --include='*.c' --include='*.cpp' --include='*.h' --include='*.hpp' \
    --include='*.swift' --include='*.ex' --include='*.exs' --include='*.dart' --include='*.sh' \
    --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules \
    --exclude-dir=__pycache__ --exclude-dir=.venv --exclude-dir=dist \
    -E '^\s*(pub(\([^)]*\))?\s+)?(fn|struct|enum|trait|class|interface|type|def|defp|defmodule|func|function|module|impl)\s+\w+' \
    "$TARGET"

# --- changes: git log ---
if git -C "$TARGET" rev-parse --git-dir &>/dev/null; then
    run_std changes git -C "$TARGET" log --oneline --stat -n 20
fi

# ============================================================
# Phase 3: output table
# ============================================================
read -r r39_l r39_w r39_b <<< "$r39_m"

pct() {
    local r39="$1" std="$2"
    if [ "$std" -eq 0 ]; then echo "-"; return; fi
    echo "$(( (std - r39) * 100 / std ))%"
}

printf '\n'
printf '%-22s %6s %7s %12s %10s\n' "Method" "Calls" "Lines" "Words(≈tok)" "Bytes"
printf '%-22s %6s %7s %12s %10s\n' "----------------------" "------" "-------" "------------" "----------"
printf '%-22s %6d %7d %12d %10d\n' "repo39 --summary" "$r39_calls" "$r39_l" "$r39_w" "$r39_b"
printf '%-22s %6d %7d %12d %10d\n' "standard method" "$std_calls" "$std_total_lines" "$std_total_words" "$std_total_bytes"
printf '%-22s %6s %7s %12s %10s\n' "savings" \
    "$(pct "$r39_calls" "$std_calls")" \
    "$(pct "$r39_l" "$std_total_lines")" \
    "$(pct "$r39_w" "$std_total_words")" \
    "$(pct "$r39_b" "$std_total_bytes")"

printf '\n'
printf '%-22s %6s %7s %12s %10s\n' "standard breakdown" "Calls" "Lines" "Words(≈tok)" "Bytes"
printf '%-22s %6s %7s %12s %10s\n' "----------------------" "------" "-------" "------------" "----------"
printf '%-22s %6d %7d %12d %10d\n' "identify" "$id_calls" "$id_lines" "$id_words" "$id_bytes"
printf '%-22s %6d %7d %12d %10d\n' "deps" "$dep_calls" "$dep_lines" "$dep_words" "$dep_bytes"
printf '%-22s %6d %7d %12d %10d\n' "map" "$map_calls" "$map_lines" "$map_words" "$map_bytes"
printf '%-22s %6d %7d %12d %10d\n' "changes" "$chg_calls" "$chg_lines" "$chg_words" "$chg_bytes"
printf '\n'
