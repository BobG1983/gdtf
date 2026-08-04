---
name: pattern-verbatim-clause-breaks-the-grep-clause
description: One clause dictates verbatim replacement prose, another requires an exhaustive `grep -rn <symbol>` to return nothing, and the dictated text contains the symbol — no build satisfies both.
metadata:
  type: feedback
---

When a ticket dictates VERBATIM replacement prose in one clause and requires an exhaustive
`grep -rn <symbol>` to "return nothing" in another, run that grep against the dictated text
itself before crediting either clause. The author almost always wrote the grep claim about
the pre-fix tree and did not notice that the replacement prose re-introduces the symbol.

**Why:** both horns are a violation, so no code change passes. Landing the dictated text
word for word fails the grep clause. Rewording the dictated text to keep the grep clean is a
deviation from dictated text, and `.claude/rules/design-fidelity.md` rule 2 requires a
deviation — including an apparent improvement — to be approved BEFORE building. An
implementer who complies perfectly with either clause is still non-compliant, which is why
this shape survives round after round: each gate sees a different horn and reads it as a new
mistake.

**How to apply:** rule the grep clause a violation and name the surviving `file:line`
(uncertainty never resolves in the implementer's favour). Then say plainly in the report that
the two clauses are mutually unsatisfiable, so the fix is a ticket amendment by the user, not
another build round — and do not soften to a PASS just because the implementer followed the
other clause literally. Amending the ticket is what actually ends it; holding the
non-compliant line is how you get there. When re-gating the same ticket, state that the
finding is unchanged rather than re-listing it as new, and diff the current tree against the
previous gate's cited lines before assuming anything moved.

Related: [[pattern-unsatisfiable-acceptance-clause]],
[[pattern-uncontracted-prose-asserts-false-history]].
