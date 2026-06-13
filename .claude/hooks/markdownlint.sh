#!/usr/bin/env bash
# PostToolUse hook: auto-fix Markdown with markdownlint-cli2 after a Write/Edit.
# Non-blocking by design — always exits 0. Acts only on *.md files; no-op if npx
# is unavailable. Config is the repo-root .markdownlint.json (discovered from cwd).
#
# NOTE: --fix resolves MD046/MD048/MD060/whitespace etc.; it CANNOT add a language
# to a bare fence (MD040) — those still surface in-editor and must be tagged by hand.

payload="$(cat)"
file="$(printf '%s' "$payload" | python3 -c '
import json, sys
try:
    print(json.load(sys.stdin).get("tool_input", {}).get("file_path", ""))
except Exception:
    print("")
' 2>/dev/null)"

case "$file" in
  *.md|*.markdown)
    if command -v npx >/dev/null 2>&1; then
      ( cd "${CLAUDE_PROJECT_DIR:-.}" && npx --yes markdownlint-cli2 --fix "$file" ) >/dev/null 2>&1 || true
    fi
    ;;
esac

exit 0
