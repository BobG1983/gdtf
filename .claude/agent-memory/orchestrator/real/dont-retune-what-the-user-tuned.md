---
name: dont-retune-what-the-user-tuned
description: The user maintains Bevy — cite source file:line and defer; tuning numbers in the RON files are deliberate design choices, not defects; "could we…" is a question, not a work order.
metadata:
  type: feedback
---

**The user is a Bevy engine maintainer.** Verify any claim about Bevy from source and cite
`file:line`. Defer to them on engine behaviour, and never overrule their dependency choices — the
workspace pins `bevy = "0.19.0"` at `Cargo.toml:27`, and that pin is theirs.

**Deliberate values are not defects.** Tuning numbers in the RON files under `assets/core_tuning/`
are design choices. The body-part hit weights favour legs over arms **on purpose** —
`assets/core_tuning/combat.tuning.ron:126-131` (head 6, torso 40, arms 12 each, legs 15 each). Do
not "correct" a tuning value, and do not pin one in a test (see [[a-flaky-test-is-a-broken-test]]).
Every tuning file carries per-line comments explaining the why — preserve them through any edit.

**A question is not a work order.** "Could we do X?" is a feasibility question to answer, not
licence to build the heaviest version of X. Answer it, give the range from minimal to full, and let
the user choose. Check whether a minimal answer exists before proposing anything large.
`.claude/rules/reply-shape.md:23` says the same thing from the other side: decide routine calls
yourself, ask only when different readings mean materially different work.

**How to apply, in chat as well as in code:**

- Lead with what changed and what you need. State the ask in the first three lines
  (`.claude/rules/reply-shape.md` rule 1).
- Name mechanisms by what they DO, not by their identifier: "some tests were being skipped
  silently" beats naming the alias and the feature flag.
- Ticket numbers are references, not sentence subjects.
- Precision belongs in contracts, gate prompts and ticket text — not progress reports.
- `.claude/rules/plain-language.md:18-33` is the wording list — read it and use the plain word it
  points to instead of the consultant one.
- Raise a concern once, then build what was asked (`reply-shape.md:25`).

Related: [[settled-calls-dont-repropose]], [[invented-names-are-jargon]].
