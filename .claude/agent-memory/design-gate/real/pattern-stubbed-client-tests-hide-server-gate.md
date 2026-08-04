---
name: pattern-stubbed-client-tests-hide-server-gate
description: A newly required step in the wire protocol can land green while the real client never sends it — the client's tests use a fake server that accepts anything.
metadata:
  type: feedback
---

When a change makes a step REQUIRED on the wire, check that the real CLIENT sends it. The
server's own tests will not tell you.

**Why:** the transport now answers `NotNegotiated` to every non-`Hello` frame until the
handshake succeeds (`crates/gdtf_net_qa_transport/src/listener/session.rs:17-30`). Mid-build,
the MCP client in `bins/gdtf_qa_mcp/src/link.rs` sent no `Hello` at all — only the readiness
probe did, on its own connection. Both of the client's suites (`tests/loopback.rs`,
`tests/reconnect.rs`) bind hand-written fake listeners that answer anything to anything, so they
stayed green while every real `mcp__gdtf-qa__*` call would have been refused.

**Closed — do not re-raise.** `Connection::negotiate` sends `Hello(ProtocolVersion::CURRENT)`
and maps a refusal to `McpError::Handshake` (`link.rs:94-98`); `ensure_connected` calls it
before storing the connection (`link.rs:150`). The fakes model the gate now:
`a_catalogue_round_trips_through_the_real_client` pins that the client's FIRST frame is the
`Hello` (`tests/loopback.rs:146-150`) and
`a_refused_handshake_fails_the_request_and_sends_nothing_else` pins the refusal path
(`loopback.rs:163-182`).

**How to apply:** grep the client for the new step (`grep -n "Hello"
bins/gdtf_qa_mcp/src/link.rs`). If the client's tests use a fake server, check the fake enforces
the gate — a fake that answers anything makes the client's compliance unfalsifiable. Whether
that is a clause violation or a blocking risk depends on what the ticket said it would change;
either way it gets reported.

Related: [[pattern-version-bump-kills-live-mcp-evidence]].
