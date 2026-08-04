---
name: pattern-optional-dep-falsifies-links-only-prose
description: docs/tooling/agent-qa.md says gdtf_qa_protocol links only serde, ron and bevy_derive; the crate carries a fourth dep and no test reads the sentence.
metadata:
  type: feedback
---

When a diff adds a dependency to `crates/gdtf_qa_protocol`, grep
`docs/tooling/agent-qa.md` for "it links only `serde`, `ron`, and `bevy_derive`"
(`:342-343`). The sentence is already false: `crates/gdtf_qa_protocol/Cargo.toml:10`
declares `schemars` as a plain dependency, and the crate's own allow-list names it
(`tests/engine_free.rs:48`).

**Why:** the allow-list test moves with the manifest because a new dep makes it red. The
doc sentence is read by nothing, so it stays behind. Any counted or enumerated claim in
`docs/` drifts the same way — see [[pattern-guard-suite-inventory-drift]] and
[[pattern-docs-mirror-router-and-manifest]].

**How to apply:** report a NOTE when the ticket has no docs clause, a VIOLATION only when
a clause requires the docs to keep step. When you propose wording, copy the shape of the
paragraph two sections down (`:359-362`): it names `ProtocolVersion::CURRENT` and the file
it lives in instead of quoting the number, so bumping
`crates/gdtf_qa_protocol/src/message/hello.rs:14` cannot falsify it. A sentence that lists
values falsifies itself; a sentence that points at the code does not.
