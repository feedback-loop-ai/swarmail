#!/usr/bin/env bash
# design-check.sh — the deterministic validation of the design sequence's
# committed spec artifacts. No network, no LLM: structural checks on the
# filesystem only. Reads its prompt file ($1) ONLY to find the result_path
# line; everything else comes from the tree (never from prior step
# results). Writes {"result": "designed"|"fail", "notes": ...} to the
# result path — "designed" only when every check passes.
set -u

prompt_file="${1:-}"
if [ -z "$prompt_file" ] || [ ! -f "$prompt_file" ]; then
    echo "design-check: prompt file argument missing or unreadable" >&2
    exit 2
fi

# The result contract renders the path as an indented line of its own:
# an absolute path ending in .json. Take the last match (the contract
# section sits at the bottom of the prompt).
result_path=""
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    case "$trimmed" in
        /*.json) result_path="$trimmed" ;;
    esac
done < "$prompt_file"
if [ -z "$result_path" ]; then
    echo "design-check: no result_path line found in the prompt file" >&2
    exit 2
fi
mkdir -p "$(dirname "$result_path")" 2>/dev/null || true

findings=""
add_finding() {
    if [ -z "$findings" ]; then findings="$1"; else findings="$findings; $1"; fi
}

# Newest specs/<slug>/ directory by modification time.
newest=""
for d in specs/*/; do
    [ -d "$d" ] || continue
    if [ -z "$newest" ] || [ "$d" -nt "$newest" ]; then newest="$d"; fi
done

ok=1
slug=""
if [ -z "$newest" ]; then
    ok=0
    add_finding "no specs/<slug>/ directory found"
else
    slug="$(basename "$newest")"
    spec_dir="specs/$slug"

    for f in spec.md plan.md tasks.md; do
        if [ ! -s "$spec_dir/$f" ]; then
            ok=0
            add_finding "$spec_dir/$f missing or empty"
        fi
    done
    # WHAT and WHY need a testable contract.
    if [ -s "$spec_dir/spec.md" ] \
        && ! grep -Eiq '^#{1,6}[[:space:]].*acceptance' "$spec_dir/spec.md"; then
        ok=0
        add_finding "$spec_dir/spec.md has no acceptance section heading"
    fi
    # Tasks are checkboxes, each paired with its proving test.
    if [ -s "$spec_dir/tasks.md" ] \
        && ! grep -Eq '^[[:space:]]*[-*][[:space:]]\[([ xX])\]' "$spec_dir/tasks.md"; then
        ok=0
        add_finding "$spec_dir/tasks.md has no checkbox task item"
    fi
    # HOW must name its risks.
    if [ -s "$spec_dir/plan.md" ] \
        && ! grep -Eiq '^#{1,6}[[:space:]].*risk' "$spec_dir/plan.md"; then
        ok=0
        add_finding "$spec_dir/plan.md has no risks section heading"
    fi
    # A semantic change must draft its decision, status proposed.
    if [ -s "$spec_dir/spec.md" ]; then
        for dec in $(grep -Eio 'decision[[:space:]]*:[[:space:]]*[0-9]{4}' "$spec_dir/spec.md" \
            | grep -Eo '[0-9]{4}' | sort -u); do
            dec_file="docs/decisions/$dec"*.md
            if ! ls $dec_file >/dev/null 2>&1; then
                ok=0
                add_finding "spec declares decision $dec but docs/decisions/$dec-*.md does not exist"
            elif ! grep -Eiq '^-[[:space:]]*\*\*Status:\*\*[[:space:]]*proposed' docs/decisions/$dec-*.md 2>/dev/null; then
                ok=0
                add_finding "decision $dec exists but is not marked status proposed"
            fi
        done
    fi
    # The committed spec must be committed: a design that exists only in
    # the worktree is not a design the implementer can read.
    if [ -d .git ] && [ -n "$(git status --porcelain -- "$spec_dir" 2>/dev/null)" ]; then
        ok=0
        add_finding "$spec_dir has uncommitted changes"
    fi
fi

if [ "$ok" -eq 1 ]; then
    result="designed"
    notes="spec artifacts for '$slug' pass every check ($findings)"
else
    result="fail"
    notes="$findings"
fi
# Sanitize for the JSON string: the pieces are fixed strings plus the
# slug, but a hostile directory name must not break out of the quotes.
notes="$(printf '%s' "$notes" | tr -d '"\\' | tr '\n' ' ')"

printf '{"result": "%s", "notes": "%s"}\n' "$result" "$notes" > "$result_path"
exit 0
