---
name: heartbeat
description: >-
  Two-hourly autonomous-build tick: ensure the cron, check the tree, judge liveness,
  resume a dead run or start exactly one new build.
argument-hint: ""
---

# /heartbeat — recovery tick for autonomous builds

Run-state is read and written by the **`/run-state` skill**, which owns the file's sections and
their formats. This tick never states a run-state format itself.
Build workflow: [`.claude/workflows/build-ticket.js`](../../workflows/build-ticket.js).
Clause audit: phase 0 of that workflow — the workflow runs it, this tick does not.

## The cron — this tick owns it

Cron jobs are session-only. They die when the session exits, so nothing survives a
restart on its own. This tick is the only thing that puts it back:

| Name | Schedule | Prompt |
|------|----------|--------|
| heartbeat | every two hours | `/heartbeat` |

**Local machine timezone.** Avoid exact hour and half-hour marks — pick an off-minute
so ticks do not pile onto the same instant as everyone else's.

The job also expires after 7 days on its own, even in a session that never restarts.
The check is presence, not age: recreate it if `CronList` does not show it, otherwise
leave it alone.

## Tick order

0. Read .claude/rules/plain-language.md
1. Check if .claude/run-state.md exists and is readable. If it does not exist or is unreadable, say so and stop. Do not start a build, and do not recreate the file from memory.
2. **Check the cron; recreate it if missing.** An empty `CronList` after a restart is the
   normal case, not a surprise.
3. **Check the rust-analyzer target dir; restore it if wiped.** See
   [Rust-analyzer target dir](#rust-analyzer-target-dir) below for what and why. Run:

   ```bash
   python3 - <<'PY'
   import json, os, subprocess
   cfg = os.environ.get("CLAUDE_CONFIG_DIR") or os.path.expanduser("~/.claude")
   f = os.path.join(cfg, "plugins/marketplaces/claude-plugins-official/.claude-plugin/marketplace.json")
   root = subprocess.run(["git", "rev-parse", "--show-toplevel"],
                         capture_output=True, text=True).stdout.strip()
   want = os.path.join(root, "target/rust-analyzer")
   if not os.path.exists(f):
       print("LSP env UNKNOWN - no marketplace.json at", f)
       raise SystemExit
   d = json.load(open(f))
   p = next((p for p in d.get("plugins", []) if p.get("name") == "rust-analyzer-lsp"), None)
   if p is None:
       print("LSP env UNKNOWN - no rust-analyzer-lsp entry")
       raise SystemExit
   srv = p.setdefault("lspServers", {}).setdefault("rust-analyzer", {})
   if srv.get("env", {}).get("CARGO_TARGET_DIR") == want:
       print("LSP env OK")
   else:
       srv.setdefault("env", {})["CARGO_TARGET_DIR"] = want
       with open(f, "w") as h:
           json.dump(d, h, indent=2, ensure_ascii=False)
           h.write("\n")
       print("LSP env RESTORED -", want)
   PY
   ```

   On `RESTORED`, say so in the tick report and note that `/reload-plugins` is needed for
   it to take. Carry on either way — this never blocks a build. On `UNKNOWN`, report the
   line verbatim and do not guess at a path.
4. **Check the tree**: current branch, `git status --porcelain`, `git worktree list`.
   Builds run on a feature branch in the MAIN repo — worktrees are banned by
   [`git-workflow.md`](../../rules/git-workflow.md). Any entry past the main repo is
   leftover from before that ban: report it, never adopt it as a live build.
5. **Liveness** — if a run is claimed in-flight, judge from the **last records in the run transcript** (or workflow status), **not** file mtime.
6. **Act:**
   - Dead run with a resume id → resume from the **main session** with Workflow `resumeFromRunId` (a workflow cannot resume another workflow).
   - Alive run → do not start a second build.
   - Nothing in flight → pick the next ticket off the queue in run-state.md check it is still open on the linear board, and if it is, start **exactly one** `build-ticket`. Do not pre-audit it by handd phase 0 fetches the live
     Linear text and audits the clauses itself, and blocks before touching status if they
     do not hold. If that ticket can't be started for any reason, report it and move on.
7. **Run `/run-state`** to write the file. It owns which sections change and how much of
   each is kept.

## Rust-analyzer target dir

rust-analyzer needs its own `CARGO_TARGET_DIR`. Without it, it shares `target/debug` with
cargo and the two block each other on the build lock — every `cargo dtest` stalls behind an
index, and every index stalls behind a build.

The setting lives in **one** place that is read: the inline `lspServers` block for
`rust-analyzer-lsp` in the marketplace's `.claude-plugin/marketplace.json`. The
`rust-analyzer-lsp` plugin also ships its own `.lsp.json`, and that file is **ignored** —
setting the env there looks correct and does nothing.

It keeps reverting because the marketplace directory is not a git checkout. Each sync
replaces it wholesale and drops the block, silently. A wiped config is invisible from the
outside: the LSP still answers, it just answers from the contended directory, so agents see
slow calls rather than an error. That is why this is a tick step and not a one-time fix.

Measured 2026-08-14: all three config homes had their marketplace copies rewritten the same
day, and the env was gone from every one.

Verify a restore took effect — env on the live process, and the directory existing:

```bash
ps eww $(pgrep -f '^rust-analyzer$' | head -1) | tr ' ' '\n' | grep CARGO_TARGET_DIR
ls -d target/rust-analyzer/
```

## Proof (operator)

Kill a run deliberately, let a tick fire, confirm the log line shows resume + run id.

For the LSP step: delete the `env` block from `marketplace.json`, let a tick fire, confirm
the report says `RESTORED` and the block is back.

## Do not

- Start multiple builds in one tick.
- Create a worktree, or read an existing one as a live build.
- Trust mtime alone for liveness.
- Launch a build with `Workflow({name: 'build-ticket'})`. A named workflow resolves once
  per session and replays that frozen copy, so edits to the file are ignored for the rest
  of the session. Use `{scriptPath: '.claude/workflows/build-ticket.js'}`.
