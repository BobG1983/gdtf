---
name: pattern-link-replaced-by-mangled-prose
description: A "re-path every intra-doc link, never delete" clause met by swapping prose in for links whose targets the same change deletes — doubled articles ("the the", "every the") are the tell.
metadata:
  type: feedback
---

When a clause says intra-doc links must be RE-PATHED and never deleted, but the same change
deletes the link TARGETS, the implementer has no legal move — and the move it usually makes is a
mechanical text substitution that leaves broken English behind.

**Why:** `broken_intra_doc_links = "deny"` (`Cargo.toml:133`) means a link to a deleted item has
to go. Replacing ``[`GangerView`](crate::view::GangerView)`` with "the roster read" inside "Handed
out by every … and echoed back" produces "Handed out by every the roster read and echoed back".
Nothing mechanical catches it: rustdoc checks links, clippy checks code, fmt checks layout.

**How to apply:** run both greps on any diff that moves or removes doc links.

```
grep -rnE "(\bthe the\b|\ba a\b|\bevery the\b|\bin in\b|\bof of\b|\bto to\b)" crates bins docs

grep -rnE --include='*.rs' '(\ban an? [a-z])|(\bthe the\b)|(\ba a\b)|(\bevery the\b)' crates bins \
  | grep -E '///|//!'
```

The first found six hits in `crates/gdtf_app/src/dev/net_qa/wire/token.rs` and `wire/misc.rs`; the
widened second found five more in `wire/misc.rs` and `wire/act.rs` that the first missed — eleven,
not six. Both return zero today, so that fix landed; keep the greps for the next link move.

Also COUNT the `crate::`-prefixed links at the base commit
(`git show develop:<file> | grep -o '(crate::[a-zA-Z_:]*'`) and compare with the number the clause
states. A clause naming "20 links" over a set that holds 23, of which 13 point at items the same
change deletes, is unsatisfiable as written and needs a ruling, not a code change.

This is also a `module-layout.md` rule 6 violation on its own (`:48-51`): a module move may edit
only visibility, import paths, and link re-pathing — "never link deletion". Cite rule 6 as well as
the clause.

Related: [[pattern-partial-falsehood-sweep]], [[pattern-guard-suite-inventory-drift]].
