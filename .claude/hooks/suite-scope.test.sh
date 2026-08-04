#!/usr/bin/env bash
# Pins suite-scope.sh path → FULL|DOCS mapping (GTW-957).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCOPE="$ROOT/.claude/hooks/suite-scope.sh"
fail=0

check() {
  local want="$1"; shift
  local got
  got="$("$SCOPE" --paths "$@")"
  if [ "$got" != "$want" ]; then
    echo "FAIL want=$want got=$got paths=$*" >&2
    fail=1
  else
    echo "PASS $want <- $*"
  fi
}

check DOCS docs/index.md
check DOCS docs/tooling/agent-qa.md
check DOCS .claude/rules/plain-language.md
check DOCS .claude/skills/gate/SKILL.md
check DOCS CLAUDE.md README.md
check DOCS docs/a.md docs/b.md .claude/rules/x.md
check FULL docs/tooling/qa-commands.md
check FULL .claude/rules/verification.md
check FULL crates/gdtf_app/src/lib.rs
check FULL docs/index.md crates/x.rs
check FULL docs/tooling/qa-commands.md docs/index.md
check FULL .claude/rules/verification.md CLAUDE.md
check FULL assets/foo.png
check FULL docs/foo.ron
check FULL .github/workflows/test.yml
# empty path list via no --paths and no git mode in a clean call with empty --paths
got="$("$SCOPE" --paths 2>/dev/null || true)"
# with zero path args after --paths, script may fall through to empty → FULL
# invoke with a deliberate empty set: no args at all
got="$("$SCOPE")"
[ "$got" = "FULL" ] && echo "PASS FULL <- (no paths)" || { echo "FAIL empty"; fail=1; }

# rename-style: non-md old name forces FULL
check FULL crates/old.rs docs/new.md

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "suite-scope.test.sh: all passed"
