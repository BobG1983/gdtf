---
name: pattern-uncontracted-prose-asserts-false-history
description: A docs fix adds unrequired attribution prose ("removed outright by ticket N") that a pickaxe search disproves — check every historical claim the diff INTRODUCES.
metadata:
  type: feedback
---

When a ticket dictates verbatim replacement text for a docs passage and the implementer writes
extra explanatory prose around it, pickaxe every symbol and attribution the NEW prose claims
history for: `git log --oneline --all -S "<symbol>" -- bins/ crates/`.

**Why:** a docs fix to the design canon added, beyond the dictated text, that `GetEditorState` "was
removed outright" by a named ticket and that a second ticket "struck its name from this file so
the retired type no longer reads as the plan". `git log --all -S GetEditorState -- bins/
crates/` returns NOTHING — the name never existed in Rust code, so nothing removed it and there
was no retired type. A change whose stated purpose was "the user can read the design record
without being told a removed type is the plan" thereby planted a fresh false claim in that
record. The ticket description repeated the same wrong belief, so trusting a ticket's history
narrative is not enough.

The false prose was struck.
`docs/tooling/qa-commands.md:21` now says only that the
`Get*QueryOptions` / `Query*(kind)` pair "was deleted", with no attribution, and `GetEditorState`
appears nowhere in `docs/`. The all-history pickaxe returns two commits, both docs-only: the
the design canon's creation and the fix itself.

**How to apply:** for any docs diff, (1) compare the landed passage against the dictated text
word for word and treat additions as uncontracted, (2) pickaxe every symbol name and ticket
attribution the addition asserts, (3) rule the clause a violation when an uncontracted addition
is false, even if every dictated element is present.

Related: [[pattern-partial-falsehood-sweep]].
