---
name: pattern-ticket-reverses-an-accepted-adr
description: A ticket builds the design an Accepted ADR explicitly rejected — read docs/decisions/ before building, and check the reversal used the `Superseded by NNNN` lifecycle instead of an edit to the old ADR's Context.
metadata:
  type: feedback
---

Before auditing a ticket that introduces an architectural mechanism, read
`docs/decisions/index.md`'s log and open any ADR that covers the same ground. An Accepted ADR
may already have rejected exactly what the ticket asks for. When it has, the reversal has to
move through the record's own lifecycle, not through a quiet edit to the rejected ADR.

**Why:** `docs/decisions/index.md:22` says an Accepted ADR is immutable — its Context and
Decision are a historical record. `:25-27` says a replacement sets the new ADR's Status to
name what it supersedes and flips the old one's Status to `Superseded by NNNN`, and the old
file stays. An implementer who instead rewrites the old ADR's Context to mention the new work
leaves the Decision still asserting the opposite, which is the conflict
`.claude/rules/design-fidelity.md` rule 4 says you must stop and surface rather than pick a
side silently.

The QA command layer is the worked example of it done right:
`docs/decisions/0007-net-qa-command-discoverability.md:10` reads `Superseded by 0008`, while
`:135` still says "We will **not** build the self-registering registry (candidate 1) now" —
the Decision was left untouched and only the Status moved.
`docs/decisions/index.md:46-47` carries the matching rows.

**How to apply:** when a ticket's central mechanism contradicts a landed ADR, check three
things: the old ADR's Status names its successor, a new ADR exists and is Accepted, and
`index.md`'s log rows match both. A Context-only edit with the Decision left contradicting the
code is a violation. Expect a frozen ADR to leave dead file paths behind — adding a path-check
skip for an immutable ADR body is legitimate, not a weakened guard.

Related: [[pattern-docs-mirror-router-and-manifest]],
[[pattern-uncontracted-prose-asserts-false-history]].
