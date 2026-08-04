---
name: pattern-docs-mirror-router-and-manifest
description: docs/tooling/agent-qa.md restates request counts, tool tables, cargo features and guard names by hand, no test compares any of them, and a correction note at the bottom does not fix the sections above it.
metadata:
  type: feedback
---

`docs/tooling/agent-qa.md` hand-copies things that live in code. Nothing tests any of it, so a copy
goes false the moment the code moves.

**Why — this has already happened and is live.** The section `## The feature + env double gate`
(:205-246) still describes a `net_qa` cargo feature chained from `bins/grimdark_turfwar/Cargo.toml`
to `crates/gdtf_app/Cargo.toml` (:210-216), a runtime gate where
`crates/gdtf_app/src/dev/net_qa/env.rs` "reads `GDTF_NET_QA`" (:224-229), and a conformance test
`crates/gdtf_test_utils/tests/binary_feature_passthrough/` that "fails the suite" (:245). All three
are gone: no `[features]` section in the workspace declares `net_qa`, `env.rs:7` is
`const fn net_qa_enabled() -> bool { true }`, and that test directory is not in
`crates/gdtf_test_utils/tests/`. A one-line note at :399 does say the feature and the env arming are
removed — 150 lines below the section it contradicts, where a reader who jumps to the heading never
sees it.

Two mirrors in the same file still match, and are the ones to re-check on any diff: the editor
router's request count (:195, against `route_editor_requests` in
`crates/gdtf_content_editor/src/net_qa/router/route.rs:11-31`), and the tool table with its
Arguments column (:266-272, against the five `ToolName` variants in
`bins/gdtf_qa_mcp/src/mcp/tools/name.rs:5-16`) plus the sentence that every tool takes the same
optional `host` argument, resolved by `resolve_host` (:250-256, against
`bins/gdtf_qa_mcp/src/mcp/courier/handle.rs:17`).

**How to apply:**

1. On any diff touching the editor router, `bins/gdtf_qa_mcp/src/mcp/tools/`, a Cargo.toml feature
   list, or a test directory name, open `docs/tooling/agent-qa.md` and check the count sentence,
   the tool rows, the gate section and every named path before crediting the clause.
2. A doc sentence naming a test — "X fails the suite if …" — is a claim about a path. Open it. A
   deleted guard leaves its promise behind in prose.
3. A correction appended at the end of a doc does not fix the prose above it. Ask for the stale
   section to be rewritten or deleted.

Related: [[pattern-doc-heading-rename-anchors]], [[pattern-cross-binary-constant-coupling]].
