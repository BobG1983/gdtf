#!/usr/bin/env bash
# cargo-alias-gate.sh — PreToolUse hook (matcher: "Bash") for gdtf.
#
# WHY THIS EXISTS: a bare `cargo test` / `build` / `clippy` / `check` / `run`
# uses a different feature set from the aliases in .cargo/config.toml. That
# rebuilds the whole graph in both directions, and drops dev_tools so
# feature-gated tests silently do not run — a green exit 0 asserting nothing.
# .claude/rules/cargo-commands.md is the rule; this makes it hold.
#
# CONTRACT (Claude Code PreToolUse hook):
#   stdin  : event JSON, e.g. {"tool_name":"Bash","tool_input":{"command":"..."}}
#   exit 0 : allow the tool call.
#   exit 2 : BLOCK the tool call; stderr is shown to the model.
#
# Parse failures fail OPEN (ALLOW): a malformed payload must not wedge every
# Bash call. The payload arrives via the environment, NOT stdin — the heredoc
# below is python's stdin.
#
# TEST OVERRIDE: none needed; the hook runs no cargo. Exercise it by piping a
# payload: echo '{"tool_input":{"command":"cargo test"}}' | cargo-alias-gate.sh

set -u

CARGO_ALIAS_GATE_PAYLOAD="$(cat 2>/dev/null || true)"
export CARGO_ALIAS_GATE_PAYLOAD
BAD="$(python3 <<'PY' 2>/dev/null
import json, os, re, shlex, sys
try:
    data = json.loads(os.environ.get("CARGO_ALIAS_GATE_PAYLOAD", ""))
except Exception:
    sys.exit(0)
cmd = (data.get("tool_input") or {}).get("command", "") or ""

# Bare subcommands that have a d-prefixed alias. Everything else — the aliases
# themselves, fmt, doc, doc-full, nextest, tree, metadata — is allowed.
BLOCKED = {"test": "dtest", "build": "dbuild", "clippy": "dclippy",
           "check": "dcheck", "run": "drun"}

# Cargo global options that take a separate value, so the value is not mistaken
# for the subcommand.
VALUE_OPTS = {"--color", "--config", "-Z", "-C"}

for seg in re.split(r"&&|\|\||;|\||\n", cmd):
    try:
        toks = shlex.split(seg)
    except Exception:
        continue
    for i, tok in enumerate(toks):
        if os.path.basename(tok) != "cargo":
            continue
        skip = False
        for nxt in toks[i + 1:]:
            if skip:                      # value of a global option
                skip = False
                continue
            if nxt in VALUE_OPTS:
                skip = True
                continue
            if nxt.startswith(("+", "-")):  # +toolchain, --offline, -q, …
                continue
            # First bare token after cargo is the subcommand. Whatever it is,
            # the scan for this segment ends here — a filter or target name
            # further along must never be read as a subcommand.
            if nxt in BLOCKED:
                print("%s %s" % (nxt, BLOCKED[nxt]))
                sys.exit(0)
            break
PY
)"

if [ -n "$BAD" ]; then
  set -- $BAD
  cat >&2 <<EOF
BLOCKED: bare \`cargo $1\`. Use \`cargo $2\` — the aliases in .cargo/config.toml
carry dynamic_linking and dev_tools. A bare run rebuilds the whole graph and
drops dev_tools, so feature-gated tests do not run and still exit 0.

One test:  cargo dtest -- <name filter>
One suite: cargo dtest --test <suite target>

See .claude/rules/cargo-commands.md.
EOF
  exit 2
fi
exit 0
