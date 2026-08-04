#!/usr/bin/env bash
# suite-scope.sh — print FULL or DOCS for a set of changed paths.
#
# FULL = run the cargo green suite. DOCS = markdown-only allowlist; skip cargo.
# Fail closed: empty path set, unknown paths, or errors → FULL.
#
# Usage:
#   suite-scope.sh --paths path [path ...]
#   suite-scope.sh --staged
#   suite-scope.sh --from <git-ref>
#   suite-scope.sh --from <git-ref> --staged   # union of both

set -u

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT" || { echo FULL; exit 0; }

paths=()
use_staged=0
from_ref=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --paths)
      shift
      while [ "$#" -gt 0 ] && [[ "$1" != --* ]]; do
        paths+=("$1")
        shift
      done
      ;;
    --staged)
      use_staged=1
      shift
      ;;
    --from)
      from_ref="${2:-}"
      if [ -z "$from_ref" ]; then
        echo FULL
        exit 0
      fi
      shift 2
      ;;
    *)
      paths+=("$1")
      shift
      ;;
  esac
done

append_lines() {
  # stdin: one path per line
  local line
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line#$'\r'}"
    [ -n "$line" ] || continue
    paths+=("$line")
  done
}

if [ "$use_staged" -eq 1 ]; then
  git diff --cached --name-only --diff-filter=ACMR 2>/dev/null | append_lines
  git diff --cached --name-status --diff-filter=R 2>/dev/null \
    | awk '{ if (NF>=3) { print $2; print $3 } }' | append_lines
fi

if [ -n "$from_ref" ]; then
  git diff --name-only --diff-filter=ACMR "$from_ref" 2>/dev/null | append_lines
  git diff --name-status --diff-filter=R "$from_ref" 2>/dev/null \
    | awk '{ if (NF>=3) { print $2; print $3 } }' | append_lines
fi

# Dedup + strip ./
if [ "${#paths[@]}" -gt 0 ]; then
  _tmp="$(mktemp)"
  printf '%s\n' "${paths[@]}" | sed 's|^\./||' | awk 'NF && !seen[$0]++' > "$_tmp"
  paths=()
  while IFS= read -r line || [ -n "$line" ]; do
    [ -n "$line" ] && paths+=("$line")
  done < "$_tmp"
  rm -f "$_tmp"
fi

if [ "${#paths[@]}" -eq 0 ]; then
  echo FULL
  exit 0
fi

is_force_full() {
  case "$1" in
    docs/tooling/qa-commands.md|.claude/rules/verification.md) return 0 ;;
    *) return 1 ;;
  esac
}

is_docs_allowlist() {
  local p="$1"
  case "$p" in
    *.md) ;;
    *) return 1 ;;
  esac
  case "$p" in
    */*) ;;
    *) return 0 ;;
  esac
  case "$p" in
    docs/*|.claude/*) return 0 ;;
    *) return 1 ;;
  esac
}

for p in "${paths[@]}"; do
  [ -n "$p" ] || continue
  if is_force_full "$p"; then
    echo FULL
    exit 0
  fi
  if ! is_docs_allowlist "$p"; then
    echo FULL
    exit 0
  fi
done

echo DOCS
exit 0
