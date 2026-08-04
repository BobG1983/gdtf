---
name: pattern-hand-written-deref-ok-in-qa-mcp
description: A hand-written impl Deref is NOT a newtype-hygiene violation in bins/gdtf_qa_mcp — the crate has no bevy dependency, so #[derive(Deref)] does not exist there.
metadata:
  type: feedback
---

A hand-written `impl Deref for <Newtype>` inside `bins/gdtf_qa_mcp` is the house style there,
not a violation of the newtype rule.

**Why:** `.claude/rules/no-bare-types.md` rule 2 (:19-21) says a newtype implements `Deref`,
and its example (:81) writes `#[derive(Deref, …)]` — a Bevy re-export. `bins/gdtf_qa_mcp`
deliberately depends on `gdtf_qa_protocol`, `serde` and `serde_json` and nothing else, so
that derive macro is not in scope for the crate. Every newtype in it hand-writes the impl:
`src/link.rs:31` and `:51`; nine impls in `src/lifecycle/values.rs` (`:17` through `:186`);
eight in `src/lifecycle/launch/values.rs` (`:18` through `:180`); `src/mcp/child_path.rs:17`
and `:35`.

**How to apply:** before condemning a manual `Deref`, read the crate's `Cargo.toml` for a
bevy dependency. Still require the PRIVATE inner field — `pub struct X(PathBuf);`, never
`pub struct X(pub PathBuf)` — because `no-bare-types.md` rule 5 (:65-71) holds everywhere and
does not depend on how `Deref` got there. All twenty impls in this crate wrap a private
inner.

Related: [[pattern-newtype-stripped-to-serve-two-callers]].
