---
name: pattern-gate-added-upstream-breaks-the-only-client
description: A change to what the QA listener admits before the handshake must be checked against every real client's first frame — test suites that stand up their own fake server stay green while every live tool call fails.
metadata:
  type: feedback
---

When a diff changes what the shared listener accepts, read the FIRST frame every
production client sends. A test that hand-rolls its server proves nothing about the real
listener.

**Why:** `crates/gdtf_net_qa_transport/src/listener/session.rs:17-22` answers any non-`Hello`
frame on a fresh connection with `QaError::NotNegotiated`; only a `Hello` carrying the
listener's own version moves the connection to `Negotiated` (`:24-26`). Every client has to
open with `Hello` or it gets nothing. The MCP bridge does:
`bins/gdtf_qa_mcp/src/link.rs:134` `ensure_connected` calls `negotiate` at `:150`, which
sends `QaRequest::Hello(ProtocolVersion::CURRENT)` at `:95`. The readiness probe is a
separate connection and sends its own `Hello` (`bins/gdtf_qa_mcp/src/lifecycle/probe.rs:30`).
A pre-handshake refusal added without that call would break every `mcp__gdtf-qa__*` tool
while the bridge's own tests passed.

**How to apply:** `grep -rn "TcpStream::connect" crates bins` lists every client in one
command — today the two in `bins/gdtf_qa_mcp` plus test clients in `gdtf_net_qa_transport`,
`gdtf_app`, and `gdtf_content_editor`. Open each and check what it writes first. When a
suite's server is a fake, ask what pins the real one:
`crates/gdtf_net_qa_transport/tests/transport/round_trip.rs:16-20` is the shape to want —
it spawns the real listener and handshakes against it, with the fake standing in only for
the host behind it.
