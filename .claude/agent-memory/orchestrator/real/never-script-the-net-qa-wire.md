---
name: never-script-the-net-qa-wire
description: Standing ban — drive the running game or editor only through the mcp__gdtf-qa__* tools; a missing or stale tool is a stop-and-report, never a script, a hand-restart, or a raw socket client.
metadata:
  type: feedback
---

Drive the running game or editor **only** through the `mcp__gdtf-qa__*` tools. Never write a
socket client, a `nc` pipeline, or any script that speaks the QA wire protocol — not in Python,
not in a shell, not "just this once for a worktree".

**Why:** a script is a private capability that dies with the session, and every one written is
evidence of a missing tool answered with a workaround instead of a fix. It also tests the wrong
thing: a hand-rolled client picks its own port and its own working directory, which is how a QA
pass gets reported against a checkout nobody reviewed. The ban is written into the build
workflow's house rules (`.claude/workflows/build-ticket.js:37`) and into
`docs/tooling/agent-qa.md`.

**How to apply:**

1. **A missing or stale tool is STOP AND REPORT.** Do not kill the resident `gdtf_qa_mcp`
   process and do not rebuild it by hand — the same house rule forbids both. Make
   "I need the mcp restarting please" the first line of your next reply, not a footnote under a
   status report, then keep working on tickets that do not need the MCP. Being blocked on one
   ticket is not being blocked on the run.
2. **You cannot reconnect it yourself.** `.mcp.json` runs
   `sh -c "cargo build -p gdtf_qa_mcp && exec target/debug/gdtf_qa_mcp"`, so every fresh spawn
   rebuilds — but the connection is the pipe pair the client made at spawn. A hand-launched copy
   holds no pipe and exits on EOF from `/dev/null`. Only the user's `/mcp` reconnects it.
3. **The old excuse for a bypass is gone.** `launch` takes `package`, `features`, `working_dir`
   and `env` (`launch_schema` in `bins/gdtf_qa_mcp/src/mcp/tools/schema.rs`), so the host can
   build and drive another checkout, including a worktree. There is no tree the tools cannot
   reach.
4. **Rust tests are a different thing.** Suites under `crates/*/tests/` and `bins/*/tests/` that
   exercise the protocol stay legitimate — they live in the repo and run in the gate. The ban is
   on out-of-repo scripts used to drive a *running* app.

Related: [[agent-qa-harness]].
