#!/usr/bin/env bash
# pre-commit-gate.sh — PreToolUse hook (matcher: "Bash") for gdtf.
#
# WHY THIS EXISTS: to make three failure modes impossible at the tool boundary,
# before git runs:
#   1. Branch guard — features branch off develop
#      (git checkout -b feature/<name>); never commit directly to
#      develop or main.
#   2. Gate-pass guard — /gate records a pass in .claude/.gate-pass (TICKET /
#      BRANCH / HEAD / FINGERPRINT / SCOPE lines). A commit is allowed only if
#      that file exists, names the CURRENT branch, and its recorded HEAD is an
#      ancestor-or-equal of the current HEAD (so /land's multiple per-concern
#      commits all pass off one gate). This hook does NOT recompute FINGERPRINT
#      — /land does, via the command in .claude/rules/verification.md. This
#      guard is the deterministic backstop that makes "commit without a
#      gate-pass is blocked" literally true.
#   3. Suite gate — the workspace green suite is the ONE definition of green.
#      A red suite blocks the commit; "it should pass" is not evidence.
#
# CONTRACT (Claude Code PreToolUse hook):
#   stdin  : event JSON, e.g. {"tool_name":"Bash","tool_input":{"command":"..."}}
#   exit 0 : allow the tool call.
#   exit 2 : BLOCK the tool call; stderr is shown to the model.
#
# TEST OVERRIDE: set PRE_COMMIT_GATE_SUITE_CMD to replace the suite command
# (e.g. =true for a guaranteed-green run, =false for guaranteed-red) so the
# gate's logic can be exercised without running cargo.

set -u

# --- Detect a commit in the Bash command (python3 assumed). ---
# Tokenizer pass: split the command on &&, ||, ;, |, newline; shlex each
# segment; find the `git` token; skip git's GLOBAL options (-C <dir>, -c <kv>,
# --git-dir, --work-tree, …, both separate-arg and = forms) to the real
# subcommand. This is what catches `git -C /tmp commit` and
# `git -c user.name=x commit`, which a bare "git commit" regex misses.
# Regex backstop: the word sequence "git commit" anywhere in the raw command
# (catches `sh -c 'git commit'`, `/usr/bin/git commit`). Conservative by
# intent: a quoted "git commit" is treated as a commit attempt.
# Parse failures of the PAYLOAD fail OPEN (ALLOW): a malformed payload must
# not wedge every Bash call — the gate only rules commands it can read.
# NOTE: the payload is handed to python via the environment, NOT stdin — the
# heredoc below IS python's stdin (it carries the script), so reading the
# payload from sys.stdin there would silently see EOF and fail open.
PRE_COMMIT_GATE_PAYLOAD="$(cat 2>/dev/null || true)"
export PRE_COMMIT_GATE_PAYLOAD
DETECT="$(python3 <<'PY' 2>/dev/null
import json, os, re, shlex, sys
try:
    data = json.loads(os.environ.get("PRE_COMMIT_GATE_PAYLOAD", ""))
except Exception:
    print("ALLOW"); sys.exit(0)
cmd = (data.get("tool_input") or {}).get("command", "") or ""

OPT_WITH_SEPARATE_ARG = {"-C", "-c", "--git-dir", "--work-tree", "--namespace",
                         "--exec-path", "--config-env", "--super-prefix"}

def analyze(tokens):
    """(is_commit, -C dir or None) for one shell segment."""
    for i, tok in enumerate(tokens):
        if tok != "git":
            continue
        c_dir, j = None, i + 1
        while j < len(tokens):
            t = tokens[j]
            if t in OPT_WITH_SEPARATE_ARG:
                if t == "-C" and j + 1 < len(tokens):
                    c_dir = tokens[j + 1]
                j += 2
            elif t.startswith("-") and t != "-":   # other globals, incl. --opt=val
                j += 1
            else:
                return (t == "commit", c_dir)
        return (False, None)
    return (False, None)

is_commit, c_dir = False, None
for seg in re.split(r"&&|\|\||[;|\n]", cmd):
    try:
        toks = shlex.split(seg)
    except ValueError:
        toks = seg.split()
    hit, d = analyze(toks)
    if hit:
        is_commit, c_dir = True, d
        break
if not is_commit and re.search(r"(^|[^\w])git\s+commit([^\w-]|$)", cmd):
    is_commit = True
print("COMMIT\t" + (c_dir or "") if is_commit else "NOCOMMIT")
PY
)" || DETECT="ALLOW"

case "$DETECT" in
  COMMIT*) ;;            # fall through to the guards
  *) exit 0 ;;           # NOCOMMIT / ALLOW / empty: fast path
esac

C_DIR="${DETECT#COMMIT}"
C_DIR="${C_DIR#?}"        # strip the tab (empty when no -C was given)
FALLBACK_DIR="${CLAUDE_PROJECT_DIR:-$PWD}"
TARGET_DIR="${C_DIR:-$FALLBACK_DIR}"   # `git -C <dir> commit` is judged against <dir>
# REPO_DIR follows TARGET_DIR's own toplevel (correct inside a linked worktree,
# where the gate-pass and the suite must run against THAT tree, not the main one)
# rather than the fixed CLAUDE_PROJECT_DIR — a worktree-rooted commit with its own
# legitimate .claude/.gate-pass was otherwise always judged against the main
# tree's gate-pass and its suite ran against the main tree's code (GTW-753).
REPO_DIR="$(git -C "$TARGET_DIR" rev-parse --show-toplevel 2>/dev/null || echo "$FALLBACK_DIR")"

# --- (1) Branch guard: never commit on develop or main. ---
BRANCH="$(git -C "$TARGET_DIR" branch --show-current 2>/dev/null || true)"
case "$BRANCH" in
  develop|main)
    echo "commit blocked: use a feature branch (git checkout -b feature/...)" >&2
    exit 2
    ;;
esac

# --- (2) Gate-pass guard: no commit without a /gate pass for this tree. ---
GATE_PASS="$REPO_DIR/.claude/.gate-pass"
if [ ! -f "$GATE_PASS" ]; then
  echo "commit blocked: no gate-pass (.claude/.gate-pass) — run /gate first" >&2
  exit 2
fi
PASS_BRANCH="$(sed -n 's/^BRANCH=//p' "$GATE_PASS" | head -n1)"
PASS_HEAD="$(sed -n 's/^HEAD=//p' "$GATE_PASS" | head -n1)"
CUR_HEAD="$(git -C "$TARGET_DIR" rev-parse HEAD 2>/dev/null || true)"
if [ -z "$PASS_BRANCH" ] || [ -z "$PASS_HEAD" ] || [ -z "$CUR_HEAD" ] \
   || [ "$PASS_BRANCH" != "$BRANCH" ] \
   || ! git -C "$TARGET_DIR" merge-base --is-ancestor "$PASS_HEAD" "$CUR_HEAD" 2>/dev/null; then
  echo "commit blocked: gate-pass is stale or for another branch/repo — re-run /gate" >&2
  exit 2
fi

# --- (3) Suite gate: pre-commit always runs the fast FULL subset. ---
# Docs-only skip is an agent decision in /gate and /land (skill prose), not
# automated here. This hook stays fail-closed: always cargo.
# Full green is the six aliases in verification.md via /gate.
# Pre-commit subset: fmt, dclippy, dtest, dbuild (four alias steps).
# Use `.cargo/config.toml` aliases — never hand-typed feature lists.
SUITE_CMD="${PRE_COMMIT_GATE_SUITE_CMD:-cargo fmt --check && cargo dclippy -- -D warnings && cargo dtest && cargo dbuild}"
SUITE_OUTPUT="$(cd "$REPO_DIR" && bash -c "$SUITE_CMD" 2>&1)"
SUITE_STATUS=$?
if [ "$SUITE_STATUS" -ne 0 ]; then
  printf '%s\n' "$SUITE_OUTPUT" | tail -n 40 >&2
  echo "commit blocked: green suite red (fmt/clippy/test/build)" >&2
  exit 2
fi

exit 0
