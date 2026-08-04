---
name: pattern-mcp-host-built-from-the-session-checkout
description: The QA MCP host is rebuilt from the session's working directory, so a feature branch can never produce live mcp__gdtf-qa__* evidence for a change to the host itself.
metadata:
  type: feedback
---

A clause demanding live `mcp__gdtf-qa__*` evidence for a change to the MCP host's own code
(`bins/gdtf_qa_mcp`) cannot be met while the work sits on a feature branch.

**Why:** `.mcp.json:5` runs `sh -c "cargo build -p gdtf_qa_mcp && exec target/debug/gdtf_qa_mcp"`
and sets no working directory, so the host is always built from the session's working directory —
the main checkout, on `develop`. A branch, worktree or not, never reaches that binary until it
lands. Two cases, and the second is sharper:

- **Stale schema.** The diff adds arguments the running tool does not advertise. `launch_schema`
  (`bins/gdtf_qa_mcp/src/mcp/tools/schema.rs:21`) builds `port`, `package`, `features`,
  `working_dir` and `env` at `:26-44`, and `host_property` at `:13` builds the shared `host`
  argument. New keys there are invisible to a session that is already running.
- **Absent tool.** The diff adds a whole tool. The session's tool list is fixed when the session
  starts, so the tool is not merely stale — it does not exist to call. Today's five are `launch`,
  `stop`, `logs`, `commands`, `run` (`bins/gdtf_qa_mcp/src/mcp/tools/name.rs:31-35`).

**How to apply:** confirm first-hand by reading your own live tool schema — the arguments your
`launch` tool advertises right now — and compare it against the diff's `schema.rs`. If the
branch's new argument or new tool is missing from the live list, the evidence was never obtained:
rule the clause NON-COMPLIANT and say the transcripts must be captured after the branch lands, or
after the host is rebuilt from that tree and the resident process killed. Never accept an
in-process Rust test as a substitute for a clause that names tool calls —
`bins/gdtf_qa_mcp/tests/lifecycle/recipe.rs:43` drives `CargoSpawner` through
`HostManager::with_config`, which proves the mechanism, not the clause.

Related: [[pattern-unsatisfiable-acceptance-clause]],
[[pattern-version-bump-kills-live-mcp-evidence]].
