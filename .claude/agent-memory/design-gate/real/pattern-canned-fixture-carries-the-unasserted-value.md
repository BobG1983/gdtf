---
name: pattern-canned-fixture-carries-the-unasserted-value
description: When a reply is proved in two places, check which one asserts the discriminating VALUE — a presence check passes on an explicit null.
metadata:
  type: feedback
---

When a reply is proved in two places — a real-listener test in the crate that produces it and
a canned-fixture test in the MCP courier — check WHICH one carries the discriminating
assertion. If the only value assertion lives in a hand-written fixture, the production read is
pinned by nothing.

**Why:** serde writes `Option::None` as an explicit `null`, so `.get(key).is_some()` is true
for a field that is always empty. A test built only from presence checks leaves every state
read and every mapping arm deletable while green. The courier fixture cannot close that:
`bins/gdtf_qa_mcp/tests/jsonrpc/support.rs:20` is a `CANNED_REPLY` string literal, so it proves
only that the courier parses what someone typed.

The current tree is the shape to ask for.
`crates/gdtf_app/tests/net_qa/commands.rs:95-114` asserts `phase["app"] == "Running"`,
`phase["running"] == "Menu"` and `phase["battlescape"] == null` over a real listener, keeping
the presence loop as an extra rather than the whole proof.
`crates/gdtf_app/tests/net_qa/app_phase_depth.rs:75-99` goes further and compares every
reported level against the app's own live `State` resource (`live_name`, `:48-52`), so no level
can report a stale or hardcoded value.

**How to apply:** for any nested or optional reply, ask what VALUE the real-path test pins. If
the only value assertion is a fixture literal, name the change that stays green — return `None`
for every nested level, or swap one mapping arm — and rule it a violation. Check the harness
first: the strong assertion is usually free, because the fixture already rests the app
somewhere specific.

Related: [[pattern-conjunction-predicate-half-pinned]].
