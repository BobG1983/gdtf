---
name: pattern-bound-listener-pins-the-handshake-guard
description: Before ruling a protocol-handshake guard untested, check whether a bound-but-not-yet-accepting TcpListener fixture already discriminates it — the kernel completes the TCP handshake into the backlog.
metadata:
  type: feedback
---

A guard of the form "only treat the port as held when it ANSWERS the QA `Hello`, not when a
socket merely accepts" looks untested when every free-port fixture is a bind-then-drop
`free_port()` (connection refused). Check the OTHER fixtures before ruling it unpinned.

**Why:** `SystemOrphanWatch::inspect`
(`bins/gdtf_qa_mcp/src/lifecycle/orphan.rs:138-143`) maps `Readiness::NotYet => PortHold::Free`.
Dropping the handshake requirement would make the MCP host SIGKILL any unrelated process
sitting on a QA port. It is pinned incidentally: `spawn_gated_fake_game`
(`bins/gdtf_qa_mcp/tests/lifecycle/support.rs:82-104`) binds the listener at `:83` and only
calls `incoming()` once the gate opens (`:92-102`), and `TcpListener::bind` already listens —
so the kernel completes connects into the backlog while nothing answers.
`launch_becomes_ready_then_stops` (`bins/gdtf_qa_mcp/tests/lifecycle/process.rs:11-35`)
launches into that port BEFORE the gate opens, so a connect-only orphan check would report
`PortHeldByOrphan` and fail it. The gate-open variant `spawn_fake_game` (`support.rs:106-110`)
is what `bins/gdtf_qa_mcp/tests/lifecycle/production_wiring.rs:8-22` uses to GET
`PortHeldByOrphan`. Between them the two fixtures discriminate the handshake requirement.

**How to apply:** when a probe or handshake check looks unasserted, ask what state each
fixture's socket is actually in — bound-not-accepting is not closed — and trace which existing
test runs while it is in that state.
