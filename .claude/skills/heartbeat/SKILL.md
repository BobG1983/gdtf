---
name: heartbeat
description: >-
  Two-hourly autonomous-build tick: ensure the cron, check the tree, judge liveness,
  resume a dead run or start exactly one new build.
argument-hint: ""
---

# /heartbeat: recovery tick for autonomous builds

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

The `/run-state` skill owns run-state's sections and formats. This tick never states one.

## The cron

Cron jobs die when the session exits. Only this tick puts them back.

| Name | Schedule | Prompt |
|------|----------|--------|
| heartbeat | every two hours | `/heartbeat` |

Schedule it in the local machine timezone. Pick an off-minute, not the hour or half-hour
mark, so ticks do not land on the same instant as everyone else's.

The job also expires after 7 days, even in a session that never restarts. Recreate it if
`CronList` does not show it, otherwise leave it alone.

## Tick order

0. Read and follow [plain-language.md](../../rules/plain-language.md).
1. If .claude/run-state.md is missing or unreadable, say so and stop. Do not recreate the
   file from memory.
2. Check the cron; recreate it if missing.
3. Check the rust-analyzer target dir; restore it if wiped. See
   [Rust-analyzer target dir](#rust-analyzer-target-dir). Run:

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

   On `RESTORED`, say so in the tick report, and note that the change needs
   `/reload-plugins`. This step never blocks a build. On `UNKNOWN`, report the line
   verbatim and do not guess at a path.
4. Check the tree: the current branch, `git status --porcelain`, and `git worktree list`.
   Builds run on a feature branch in the MAIN repo, because
   [`git-workflow.md`](../../rules/git-workflow.md) bans worktrees. Report any entry past the
   main repo. Never adopt one as a live build.
5. If a run is claimed in-flight, judge liveness from the last records in the run
   transcript or the workflow status. Do not go by file mtime.
6. Act:
   - Dead run with a resume id: resume from the main session with Workflow
     `resumeFromRunId`. A workflow cannot resume another workflow.
   - Alive run: do not start a second build.
   - Nothing in flight: pick the next ticket off the queue in run-state.md, check it is
     still open on the Linear board, and if it is, start exactly one `build-ticket`. Do not
     pre-audit that ticket by hand. Phase 0 fetches the live Linear text, audits the clauses
     itself, and blocks before touching status if they do not hold. If the ticket cannot be
     started, report that and move on.

   Never launch a build with `Workflow({name: 'build-ticket'})`. A named workflow resolves
   once per session and replays that frozen copy, so later edits to the file are ignored.
   Use `{scriptPath: '.claude/workflows/build-ticket.js'}`.
7. Call `LSP` with a workspace symbol to ensure the LSP cache is warm.
8. Run `/run-state` to write the file.

## Rust-analyzer target dir

rust-analyzer needs its own `CARGO_TARGET_DIR`. Without it, it shares `target/debug` with
cargo and the two block each other on the build lock. Every `cargo dtest` then stalls behind
an index, and every index stalls behind a build.

The setting lives in one place that is read: the inline `lspServers` block for
`rust-analyzer-lsp` in the marketplace's `.claude-plugin/marketplace.json`. The
`rust-analyzer-lsp` plugin also ships its own `.lsp.json`. Setting the env there looks
correct and does nothing.

The marketplace directory is not a git checkout. Each sync replaces the directory wholesale
and drops the block. The LSP still answers, just from the contended directory, so agents see
slow calls rather than an error.

Verify a restore took effect:

```bash
ps eww $(pgrep -f '^rust-analyzer$' | head -1) | tr ' ' '\n' | grep CARGO_TARGET_DIR
ls -d target/rust-analyzer/
```

## Proof for the operator

Kill a run deliberately, let a tick fire, and confirm the log line shows the resume and the
run id.

For the LSP step: delete the `env` block from `marketplace.json`, let a tick fire, and
confirm the report says `RESTORED` and the block is back.
