---
name: pattern-is-absolute-guard-is-dead-under-join
description: An is_absolute() early-return before a Path::join is behaviorally dead — join already discards the base for an absolute argument — so no test can discriminate it.
metadata:
  type: feedback
---

A path-resolution helper that early-returns on `reported.is_absolute()` before doing
`dir.join(reported)` has a branch no test can pin: `Path::join` with an absolute argument returns
that argument and discards the base. Delete the guard and nothing observable changes.

**Why:** `resolve_child_path` ships exactly this shape.
`bins/gdtf_qa_mcp/src/mcp/child_path.rs:49-51` returns early when `reported_path.is_absolute()`,
and `:53` is `dir.join(reported_path)`, which returns the same value. Not a violation — the clause
it answers is about resolving RELATIVE paths, and the guard states the intent — but a test named
for the absolute case passes with the guard deleted, so it proves nothing about that branch.

**How to apply:** when a clause's evidence is "this test covers the absolute case", check whether
the fallback path produces the same result. Report it as a note, not a violation, unless the
ticket asked for the branch itself.
