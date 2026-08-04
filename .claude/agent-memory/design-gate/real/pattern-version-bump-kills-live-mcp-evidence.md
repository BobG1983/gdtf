---
name: pattern-version-bump-kills-live-mcp-evidence
description: A change that bumps ProtocolVersion::CURRENT cannot also produce live mcp__gdtf-qa__* evidence — the resident MCP server was built from develop and the handshake demands exact equality.
metadata:
  type: feedback
---

A change that moves `ProtocolVersion::CURRENT` makes EVERY live MCP round trip on that branch
impossible — not just for the commands it adds. Rule such a clause a violation and demand it be
rewritten as a headless or real-socket test.

**Why:** four links, all readable without running anything.

1. `.mcp.json` spawns the MCP server as `cargo build -p gdtf_qa_mcp && exec
   target/debug/gdtf_qa_mcp` — relative paths, so its cwd is the MAIN checkout, on `develop`.
2. `bins/gdtf_qa_mcp/src/link.rs:95` sends `Hello(ProtocolVersion::CURRENT)` from `negotiate`,
   called at `:150` inside `ensure_connected` — the value compiled into THAT binary.
3. The child answers from its own compiled value: `NET_QA_PROTOCOL_VERSION =
   ProtocolVersion::CURRENT` at `crates/gdtf_app/src/dev/net_qa/config.rs:10`, so a
   feature-branch child speaks the bumped number.
4. `SessionState::admit` admits only on `*client_version == facts.protocol`
   (`crates/gdtf_net_qa_transport/src/listener/session.rs:24-28`), else
   `QaError::VersionMismatch`. `ensure_connected` then errors, so `commands` and `run` both fail
   for that child.

`launch`'s `working_dir` argument (`bins/gdtf_qa_mcp/src/mcp/tools/schema.rs:37`) does NOT
rescue this. It does make a feature branch's NEW COMMANDS visible — `Catalogue` and `Run` are
forwarded generically (`bins/gdtf_qa_mcp/src/mcp/courier/handle.rs:80,97`) and
`mcp/courier/attach.rs:13-17` renders a PNG attachment with no per-command edit — so "a new
command cannot appear in the connected catalogue" is the WRONG reason to block. The version
number is the real one.

**Second, independent limit:** `LINK_TIMEOUT` (`bins/gdtf_qa_mcp/src/link.rs:17`, 10s) is the
RESIDENT server's compiled value. Any live demo needing a longer round trip than that is
impossible even with no version bump, because a raised value only takes effect after the branch
lands AND the MCP restarts.

**How to apply:** on any QA-protocol change, grep the diff for `ProtocolVersion::CURRENT` and
read `LINK_TIMEOUT` first. If either moves, every "evidence: a live `mcp__gdtf-qa__run` call"
clause is dead on arrival.

Related: [[pattern-cross-binary-constant-coupling]],
[[pattern-stubbed-client-tests-hide-server-gate]].
