# pre-commit-gate.sh — dry-run validation transcript

Hook under test: `/Users/bgardner/dev/gdtf/.claude/hooks/pre-commit-gate.sh`
(PreToolUse hook, matcher `Bash`). Contract: **exit 0 = allow**, **exit 2 =
block** (stderr shown to the model).

## Method

A throwaway fixture repo was created and the hook driven with synthetic
PreToolUse payloads. No cargo ran — the suite step was stubbed via the
`PRE_COMMIT_GATE_SUITE_CMD` seam (`=true` ⇒ guaranteed-green, `=false` ⇒
guaranteed-red).

Payload shape fed to the hook:

```json
{"tool_name":"Bash","tool_input":{"command":"..."}}
```

NOTE ON THE SEAM: the hook reads the event JSON from **stdin** (line 44:
`PRE_COMMIT_GATE_PAYLOAD="$(cat …)"`), which then exports it under the name
`PRE_COMMIT_GATE_PAYLOAD` for the embedded python to read from the environment.
The env var is *populated from stdin* — so the payload must be piped to the
hook on stdin (pre-setting the env var alone would be overwritten by `cat`).
This driver pipes each payload on stdin, matching how Claude Code invokes the
hook.

Fixture setup (run once):

```bash
rm -rf /tmp/gtw-hook-fixture
mkdir -p /tmp/gtw-hook-fixture/.claude
git -C /tmp/gtw-hook-fixture init -q
git -C /tmp/gtw-hook-fixture config user.email "fixture@test.local"
git -C /tmp/gtw-hook-fixture config user.name "Fixture"
git -C /tmp/gtw-hook-fixture checkout -q -b develop
git -C /tmp/gtw-hook-fixture commit -q --allow-empty -m "init"   # HEAD exists
git -C /tmp/gtw-hook-fixture checkout -q -b feature/gtw-999-fixture
```

Environment for every case: `CLAUDE_PROJECT_DIR=/tmp/gtw-hook-fixture`.
A valid `.gate-pass` (used in cases c, d, e) is:

```text
TICKET=GTW-999
BRANCH=feature/gtw-999-fixture
HEAD=<current HEAD sha>
FINGERPRINT=deadbeef
```

python3 available in env: `Python 3.14.3` (the detector requires python3).

## Result matrix — required cases (a)–(f)

| Case | Scenario | Seam | Expected | Actual | Verdict |
|------|----------|------|----------|--------|---------|
| (a) | `git commit -m x` while on **develop** | green | 2 BLOCK | 2 | PASS |
| (b) | feature branch, **NO** `.gate-pass` | green | 2 BLOCK | 2 | PASS |
| (c) | feature + valid `.gate-pass` (BRANCH=branch, HEAD=HEAD) + seam **green** | green | 0 ALLOW | 0 | PASS |
| (d) | same as (c) but seam **red** | red | 2 BLOCK | 2 | PASS |
| (e) | `git status` (non-commit command) | green | 0 ALLOW | 0 | PASS |
| (f) | compound `echo hi && git commit -m x` on feature branch w/o gate-pass | green | 2 BLOCK | 2 | PASS |

Raw transcript:

```text
=== (a) git commit on develop -> BLOCK ===
expected=2 actual=2 -> PASS
stderr: commit blocked: use a feature branch (git flow feature start ...)

=== (b) feature branch, NO .gate-pass -> BLOCK ===
expected=2 actual=2 -> PASS
stderr: commit blocked: no gate-pass (.claude/.gate-pass) — run /gate first

=== (c) feature + valid gate-pass + seam green -> ALLOW ===
expected=0 actual=0 -> PASS
(no stderr)

=== (d) feature + valid gate-pass + seam red -> BLOCK ===
expected=2 actual=2 -> PASS
stderr: commit blocked: green suite red (fmt/clippy/test)

=== (e) git status (non-commit) -> ALLOW ===
expected=0 actual=0 -> PASS
(no stderr)

=== (f) compound 'echo hi && git commit -m x' no gate-pass -> BLOCK ===
expected=2 actual=2 -> PASS
stderr: commit blocked: no gate-pass (.claude/.gate-pass) — run /gate first
```

## Additional edge cases exercised (all PASS)

These confirm the guard logic beyond the required matrix:

| Edge case | Expected | Actual |
|-----------|----------|--------|
| Malformed payload (`not json at all`) — must **fail OPEN** | 0 ALLOW | 0 |
| gate-pass HEAD is an **ancestor** of a newer HEAD (one gate covers /land's multiple commits) | 0 ALLOW | 0 |
| gate-pass `BRANCH=` names a **different** branch | 2 BLOCK | 2 |
| gate-pass `HEAD=` is **not an ancestor** (all-zeros sha) | 2 BLOCK | 2 |
| `git -C <fixture> commit` judged against the **-C dir** (which is on develop) | 2 BLOCK | 2 |
| `git -c user.name=x commit` (global `-c` opt before subcommand) still detected as commit | 2 BLOCK | 2 |

## Defects found

None. All required cases (a)–(f) produced the expected exit code, and all
additional edge cases behaved per the hook's documented contract (branch
guard, gate-pass guard incl. ancestor logic, suite gate via seam, fail-open on
unparseable payload, and `git -C` / `git -c` detection).

## Cleanup

`/tmp/gtw-hook-fixture` was removed after the run.
