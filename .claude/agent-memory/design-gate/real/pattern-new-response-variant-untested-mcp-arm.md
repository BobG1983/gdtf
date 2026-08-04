---
name: pattern-new-response-variant-untested-mcp-arm
description: A new reply-enum variant changes what the MCP host hands the client; the compiler forces the arm only where the match is exhaustive, and never forces a test.
metadata:
  type: feedback
---

Adding a variant to a reply enum changes client-facing behaviour. Check three things: that
an arm exists, what it returns, and that a test names it.

**Why:** `render_outcome` (`bins/gdtf_qa_mcp/src/mcp/courier/run.rs:72-104`) matches
`CommandOutcome` (`crates/gdtf_qa_protocol/src/command/outcome.rs:12-39`) exhaustively —
`Ran` renders `isError: false` plus the attachment blocks, and `Unavailable`,
`BadArguments`, `Unknown` each go to `tool_error` (`bins/gdtf_qa_mcp/src/mcp/content.rs:10`,
which sets `isError: true`). All four are pinned by name in the compiled integration tests
(`bins/gdtf_qa_mcp/tests/jsonrpc/courier_tools.rs:81-134`,
`courier_riders.rs:15-31`), so a fifth arm with no test is drift from an established
convention, not a missing convention.

Two places nothing is forced. The courier's response match has catch-all arms
(`mcp/courier/handle.rs:84,101`), so a new `QaResponse` variant compiles straight through
and reaches the client as "the host answered a Run request with …" — no compile error at
all. And the launch/stop/logs renderers (`mcp/control/render.rs:18-34`, `:127-138`,
`:159-177`) currently have no compiled tests: `mcp/control/mod.rs` declares only `handle`
and `render`, so `src/mcp/control/test/` is in no build target and its four files never run.

**How to apply:** for a new variant, ask for the arm's shape — the error flag, the carried
detail relayed to the client, and no content block the client could read as a success
payload (never hand back a frame that looks like a screenshot for a refusal). Then confirm
the test module is actually declared: a `test/` directory no `mod` names compiles nowhere,
and `cargo test` never reports tests it was never given.
Related: [[pattern-untested-mcp-description-prose]].
