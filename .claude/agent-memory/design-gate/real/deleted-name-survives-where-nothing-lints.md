---
name: deleted-name-survives-where-nothing-lints
description: A "nothing references the deleted type/module/tool" claim proved by clippy + build + both cargo doc runs is unproven — none of them reads a doc comment in a private module, a cfg(test) doc link, backticked prose, a Cargo.toml comment, a rule file, or a docs/ index line.
metadata:
  type: feedback
---

`cargo dclippy`, `cargo dbuild`, `cargo doc --workspace --no-deps` and `cargo doc-full` all
pass while a deleted name is still referenced in text. Grep every deleted NAME across
`crates bins docs .github .claude` and treat a hit inside a `///`, a `//!`, or prose as
unfixed. Eight categories, none of them compiler-checked:

1. **Backticked prose in a `//!` or `///`** — not a link, so rustdoc never resolves it.
2. **A doc link inside a `#[cfg(test)]` module** — `cargo doc` does not build cfg(test), so a
   genuinely broken `crate::` link there passes both doc runs.
3. **A doc link inside a private module** — rustdoc does not document a private subtree and
   does not resolve its intra-doc links, so a `///` link to a type the same commit deleted
   stays green. Check the whole visibility chain, not just the module's own keyword.
4. **A test's own doc comment describing the old drive path** after the body was rewritten.
   The module doc gets updated, the per-test doc does not. This includes a test-support
   fixture whose doc claims it makes the same call production makes, after the production
   path stopped making it.
5. **`Cargo.toml` dependency comments** — prose, linted by nothing.
6. **`docs/index.md`'s one-line entry for the page that was rewritten** — a counted or
   feature-named claim outlives the page it summarises.
7. **`.claude/rules/*.md`** — always-loaded rules that cite crate paths and counts. Nothing
   lints a rule file.
8. **A `docs/` page the change never names, whose present-tense claim the deletion
   falsifies.**

**Why:** category 3 is not hypothetical — the two biggest QA modules are private.
`crates/gdtf_content_editor/src/lib.rs:19` declares `mod net_qa;`, and the game side is
private twice over: `crates/gdtf_app/src/lib.rs:10` `mod dev;` plus
`crates/gdtf_app/src/dev/mod.rs:8` `pub(crate) mod net_qa;`.

Two live survivors today, both from the same QA-API deletion:

- `docs/index.md:41` still says the QA channel is gated by "the `net_qa` feature +
  `GDTF_NET_QA` env double gate". The env var is real
  (`bins/gdtf_qa_mcp/src/lifecycle/launch/channel.rs:5`) but there is no `net_qa` cargo
  feature any more — `.claude/rules/verification.md:34` says the QA modules compile under
  `debug_assertions`. That is category 6. The same line's "the five MCP tools" is still
  correct (`bins/gdtf_qa_mcp/src/mcp/tools/name.rs:18-24` lists exactly five), so check each
  half of a claim separately.
- `docs/ui-picking-arbitration.md:259-262` still says "`net_qa` has no mouse-click intent on
  the wire (only `PressKey` / `Hover` / `SetFocus`; `Inject` is battle-only)". None of those
  four names exist in `crates/gdtf_qa_protocol` or `bins/gdtf_qa_mcp`. That is category 8,
  and a grep for the deleted TOOL names misses it entirely.

Fixing only the hits a reviewer named is what turns a two-sentence correction into five
review rounds.

**How to apply:** for every deleted module, type, tool, and request-variant name, grep the
four trees and walk the categories one at a time. Two greps find what a tool-name grep does
not: grep the deleted names against `--include=Cargo.toml`, and grep the deleted
REQUEST-VARIANT names across all of `docs/` and `.claude/` (exclude only immutable the design canon
bodies). Also check the doc sentence the change's own code edit falsifies — a doc comment
updated inside the diff while a `docs/` page repeating the same sentence is not.

Related: [[pattern-partial-falsehood-sweep]], [[pattern-guard-suite-inventory-drift]],
[[pattern-compiler-enforced-claim-not-enforced]].
