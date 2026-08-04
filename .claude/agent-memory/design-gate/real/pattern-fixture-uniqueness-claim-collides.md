---
name: pattern-fixture-uniqueness-claim-collides
description: A clause requiring test fixtures to hold distinct values is checked by reading every constant's VALUE across the crate — no test compares them, and a comment claiming uniqueness is not evidence.
metadata:
  type: feedback
---

When a clause says two test fixtures must stop sharing one value (so a hardcoded constant
in the code under test cannot satisfy both), verify it by listing every constant of that
kind and comparing the values. Names, and comments asserting uniqueness, prove nothing.

**Why:** the scale-factor fixtures in `crates/gdtf_content_editor` are spread over six
files and two test kinds — in-crate `src/net_qa/screenshot/test/ordering.rs:29` (1.25),
`src/net_qa/present/test/harness.rs:15` (1.5), `src/net_qa/screenshot/test/aim.rs:16`
(2.5), `src/net_qa/screenshot/test/source.rs:53` (3.0), and the integration harnesses
`tests/net_qa_editor_screenshot/harness.rs:22` (1.75) and
`tests/net_qa_editor_present/harness.rs:19` (2.0). They compile into different binaries,
so no test can compare them. The set stays distinct only because someone re-reads it.

Asking for a comment that explains the choice is also the wrong fix:
`.claude/rules/comment-hygiene.md` bans design rationale in comments.

**How to apply:** grep the whole constant kind (`grep -rn "SCALE_FACTOR\|set_scale_factor"
crates bins`), sort the hits by value, and report any two that match. Two fixtures sharing
a value means a hardcoded constant in the production path passes both tests. Related:
[[pattern-guard-suite-inventory-drift]].
