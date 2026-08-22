# Design fidelity: build what was specified

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Why this rule exists: narrowing or descoping a user-specified design without asking is this
kit's number-one failure mode. One agent "right-sized" a hard resolution to something simpler.

## Rules

1. Build EXACTLY what the user or the Linear ticket specifies. Never narrow, simplify or
   substitute a design because the specified one is harder or slower to build.
2. Propose any deviation to the user and get it APPROVED BEFORE building. That includes one
   you believe is an improvement. A silent deviation is a defect: file it with `/file-bug`.
3. Linear "Done" is NOT evidence the work exists or matches its spec. Audit the code against
   the ticket text before building on it.
4. `docs/` is the contract ALONGSIDE the ticket. The design canon is `docs/pillars/`,
   `docs/combat/` and `docs/architecture.md`. When code and docs disagree, stop and say so
   rather than picking a side.
5. Ambiguous spec? Ask the user, or route the question through the `design-gate` agent. Never
   resolve ambiguity by choosing the cheapest interpretation. For example, a ticket says "the
   sim resolves hits along an arbitrary attack vector". Do NOT collapse that to axis-aligned
   or grid-snapped ECS queries. Build the specified vector math.

## Descoping protocol (the ONLY legal way to shrink scope)

- Shrinking scope adds one step to rule 2: update the TICKET before writing any code.
- Some phrases are stop signals: "for now", "simplified version", "basic implementation", "we
  can defer", "out of scope" (applied to an in-contract clause), "MVP of this". Catching
  yourself writing one means stop and ask.
- A ticket lands only when EVERY clause is implemented. There is no "MVP of a ticket" unless
  the USER splits it.

## Enforcement

`/gate` runs the `design-gate` agent. It audits the diff against the ticket and `docs/` before
any commit. A change that passes the green suite but deviates from spec FAILS the gate.
