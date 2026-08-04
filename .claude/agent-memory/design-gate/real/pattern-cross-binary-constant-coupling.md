---
name: pattern-cross-binary-constant-coupling
description: A bin that cannot depend on the crate it launches copies that crate's constants, and each side's test pins its own copy — so the pair drifts, or one half dies, with the suite green.
metadata:
  type: feedback
---

When two packages must agree on a value and neither can depend on the other, check whether the
value is declared ONCE in a crate both import. If it is copied, nothing in the repo couples the
copies.

**Why:** `bins/gdtf_qa_mcp` cannot depend on `gdtf_app` or `gdtf_content_editor`. The listen ports
were copied on both sides until they moved into `crates/gdtf_qa_protocol/src/ports.rs:4,7`
(`GAME_QA_PORT`, `EDITOR_QA_PORT`), which all three now import — the MCP host at
`bins/gdtf_qa_mcp/src/hosts/host.rs:3`, the game at `crates/gdtf_app/src/dev/net_qa/config.rs:6,17`,
the editor at `crates/gdtf_content_editor/src/net_qa/config.rs:6,14`. That is the shape to ask for.

The env var names show what happens without it.
`bins/gdtf_qa_mcp/src/lifecycle/launch/channel.rs:5-11` still declares `GDTF_NET_QA`,
`GDTF_NET_QA_PORT`, `GDTF_EDITOR_NET_QA` and `GDTF_EDITOR_NET_QA_PORT`, and its test at :63-70
asserts them against string literals in the same file — green forever. But
`bins/gdtf_qa_mcp/src/lifecycle/spawn.rs:51` throws the channel away
(`let _ = (port, spec.channel());`) and neither child reads an env var any more:
`crates/gdtf_app/src/dev/net_qa/env.rs:7` and `crates/gdtf_content_editor/src/net_qa/env.rs:7` are
both `const fn … -> bool { true }`. The names, their newtype and their tests outlived the thing they
configured, and nothing went red.

**How to apply:**

1. For any value a launcher shares with the process it launches, ask where the ONE declaration is.
   A test asserting a constant against a literal in the same file proves nothing about the far side.
2. When the value is copied, name the mutation plainly: rename it on the child and the whole suite
   still passes while the launcher can never reach a listener.
3. Ask the reverse question too — who READS this? A launcher-side constant with no reader left is
   dead weight the tests are actively defending.

Related: [[pattern-docs-mirror-router-and-manifest]].
