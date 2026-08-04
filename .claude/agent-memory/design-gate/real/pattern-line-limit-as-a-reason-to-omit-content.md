---
name: pattern-line-limit-as-a-reason-to-omit-content
description: A module doc drops contracted content and blames the 300-line-per-file limit; module-layout rule 1 turns an outgrown concern into a directory, so the excuse is never valid.
metadata:
  type: feedback
---

"It would not fit in 300 lines" is never a reason to omit content the ticket asked for.
`.claude/rules/module-layout.md:15-18` (rule 1): a concern that outgrows one file becomes a
directory module — `<name>/mod.rs` plus focused submodules. The line band forces a SPLIT, never a
deletion.

**Why:** a ticket listed `log.rs — the act-log wire types`. The shipped file kept only the
provenance enum and two scalars and wrote the reason into its own module doc: "one 26-variant
mirror cannot live inside this ticket's 300-line-per-file limit, and a half-mirror would be worse
than none." The size excuse is the tell that the call was made for convenience.
`.claude/rules/design-fidelity.md:34-37` puts the ticket first — a scope decision recorded in a
`//!` comment is silent descoping, not an approved deviation. The directory split was the right
answer and is what the tree carries now: `crates/gdtf_app/src/dev/net_qa/wire/` is a directory
(`act.rs`, `act_payload.rs`, `cell.rs`, `key.rs`, `log.rs`, `misc.rs`, `phase.rs`, `pointer.rs`)
with `log.rs` at 49 lines.

**How to apply:** grep every new or changed module doc for the line limit cited as a reason —
`grep -rn "cannot live inside\|300-line\|would not fit" crates bins docs`. It returns nothing
today; any hit is a scope decision that needed approval before the build. Rule the omission a
violation and demand either the directory split or a ticket amendment.
