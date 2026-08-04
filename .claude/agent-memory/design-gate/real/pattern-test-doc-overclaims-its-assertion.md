---
name: pattern-test-doc-overclaims-its-assertion
description: A test's doc comment can promise a guarantee its assertions cannot observe — read the comment as a claim and name the line that proves it.
metadata:
  type: feedback
---

Read every test's doc comment as a claim, then find the assertion line that proves it. Words
that a value comparison cannot observe: "every", "always", "derived from", "at build time",
"in order", "still".

**Why:** the comment is what the next reader trusts instead of re-reading the asserts, so a
comment that overshoots quietly retires the coverage question. Two live examples:

- `crates/gdtf_app/src/dev/net_qa/wire/test/scalars.rs:55` says "Every `#[serde(transparent)]`
  scalar rides the wire as its BARE inner value". The test below it (`:57-72`) checks eleven
  types by hand, while `wire/` declares 23 `#[serde(transparent)]` types
  (`grep -rc 'serde(transparent)' crates/gdtf_app/src/dev/net_qa/wire/*.rs`). Nothing else
  covers the twelfth: the directory-scanning guards in `wire/test/coverage.rs` require a
  round-trip case (`:174-183`) and a schema case (`:186-195`) per type, and a round trip
  cannot see a lost `#[serde(transparent)]` — which is what `wire/test/drive.rs:50` says. That
  comment is also a fragment, breaking off at "not a", which is exactly why reading it as a
  claim catches what skimming does not.
- `bins/gdtf_qa_mcp/tests/jsonrpc/protocol.rs:5` says `initialize` "echoes the requested
  protocol version and reports the server identity". The echo half is pinned at `:13`; the
  identity half is only `serverInfo["name"].is_string()` at `:15`, which any string satisfies.

**How to apply:** if no line can observe the property, the fix is to narrow the comment to
what the asserts see, or to add the assertion that makes the claim true. For "every", the
repo's own answer is an enumeration guard that scans the source directory and fails per
uncovered type — `wire/test/coverage.rs:57-67` collects the declared types, `:174-195` asserts
each one appears in a case.

Related: [[pattern-tautological-assertion-replacing-a-pin]],
[[pattern-untested-mcp-description-prose]].
