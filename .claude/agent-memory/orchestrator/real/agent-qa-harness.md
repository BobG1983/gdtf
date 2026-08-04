---
name: agent-qa-harness
description: Driving the running game or editor — the five MCP tools, the host's own command catalogue as the only truth about what it can do, and why a tool description you read back can be stale.
metadata:
  type: feedback
---

Never state that a QA capability exists because a ticket, a doc, or a memory (including this one)
says so. Read `bins/gdtf_qa_mcp/src/mcp/tools/name.rs` for the tools, and ask the running host
itself what it can do.

**Why:** the tool vocabulary has already been replaced wholesale once. The twelve fixed game
tools an earlier version of this note described — `app_flow`, `query_state`, `send_input`,
`get_output`, the screenshot and stepper tools — no longer exist, along with the compact-RON
argument format, the 93k-character state dump, and the three-clocks and input-gate gotchas that
came with them. What a host can be asked to do is now read at run time, not fixed in the tool
list, so a doc can only ever describe yesterday's build.

**How to apply:**

- **Five tools.** `launch`, `stop`, `logs`, `commands`, `run` (`ToolName` in
  `bins/gdtf_qa_mcp/src/mcp/tools/name.rs`). Every one takes an optional `host` — `"game"` (the
  default, port 7616) or `"editor"` (7617), the constants in
  `crates/gdtf_qa_protocol/src/ports.rs`. Both children can be up at once and are stopped
  independently.
- **Start with `commands`.** Its reply is that host's live catalogue — one row per command with
  its name, summary, timing, and whether it can run right now. `detail: "Full"` adds each
  command's derived argument and reply schemas; those are what you read to build a `run` call.
  `run`'s `arguments` is a **JSON object shaped by that schema**, not a RON string.
- **Expect a small catalogue.** The game publishes one command today, `app.phase`
  (`GAME_COMMANDS` in `crates/gdtf_app/src/dev/net_qa/commands/set.rs`); the editor publishes
  none and answers every `run` with `Unknown`. That is the state of the build, not a fault.
- **There is no `net_qa` cargo feature and no env arming.** QA modules compile under
  `debug_assertions` and always listen on the shared ports — `net_qa_enabled()` in
  `crates/gdtf_app/src/dev/net_qa/env.rs` is `const fn ... { true }`. Anything describing a
  feature flag or a `GDTF_NET_QA` gate is out of date.
- **Never judge a reconnect by reading a tool's description.** That reads the session's cached
  schema, not the server, and it will report a successful reconnect as a failure. Check the
  binary — `pgrep -fl gdtf_qa_mcp`, `ls -la target/debug/gdtf_qa_mcp` against
  `git log -1 --format=%cd`. Tool *replies* come from the live server and are trustworthy.
- **Do not assume a reconnect killed the child.** The stdio loop stops the children it owns on
  EOF (`run_stdio` in `bins/gdtf_qa_mcp/src/serve.rs`), but a host that is killed outright leaves
  the child holding the port. `stop` detects that as an orphan and stops it; `launch` into a held
  port refuses and names it (`bins/gdtf_qa_mcp/src/lifecycle/orphan.rs`). Call `stop` first
  rather than guessing.
- **Warm the build before you launch.** `launch` runs `cargo run` and waits for the QA channel;
  the boot timeout is 180 seconds for both hosts (`bins/gdtf_qa_mcp/src/lifecycle/config.rs`).
  Run `cargo dbuild` before launching the game and `cargo edbuild` before
  `launch(host: "editor")`, or the first call spends its whole budget compiling. If a launch does
  time out, read `logs` — both of the child's output streams are captured in order.
- **Read cargo test totals from raw output.** A log filter between you and cargo can undercount;
  judge by exit code and the raw `test result:` lines.

Related: [[never-script-the-net-qa-wire]].
