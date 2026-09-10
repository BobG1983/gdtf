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
4. `docs/` is the contract ALONGSIDE the ticket. The current design canon is `docs/`.
   Tickets will change the design and may disagree with `docs/`. The ticket is the source of truth for the current change, `docs/` will be updated accordingly.
5. Ambiguous spec? Ask the user, or route the question through the `design-gate` agent. Never
   resolve ambiguity by choosing the cheapest interpretation. For example, a ticket says "the
   sim resolves hits along an arbitrary attack vector". Do NOT collapse that to axis-aligned
   or grid-snapped ECS queries. Build the specified vector math.

6. A design proposal is written against `docs/` and the ticket. The current implementation is
   not a constraint unless `docs/` says it is. Citing existing code as a reason to keep a shape
   is a deviation under rule 2, and needs the same approval before building.

   Measured on GTW-702. A software design proposal anchored on the existing stand-in AI as though
   replacing it were out of bounds, treating code that was always meant to be temporary as the
   thing the design had to preserve.

## Descoping protocol (the ONLY legal way to shrink scope)

- Shrinking scope adds one step to rule 2: update the TICKET before writing any code.
- Some phrases are stop signals: "for now", "simplified version", "basic implementation", "we
  can defer", "out of scope" (applied to an in-contract clause), "MVP of this". Catching
  yourself writing one means stop and ask.
- A ticket lands only when EVERY clause is implemented. There is no "MVP of a ticket" unless
  the USER splits it.

## The carried fix: a defect that blocks a clause of the ticket being built

A build that finds a defect blocking a clause of the ticket it is building may fix it, instead of
filing the defect and stopping. Measured on GTW-1017, whose build could not show its own
determinism clause green until the presenter's GPU lock race was fixed. One ticket became two for a
change of a few lines.

Four bounds hold on every carried fix, all of them required:

- The defect blocks a clause of the ticket being built, and the build says which clause. A defect
  that blocks no clause is reported, not fixed.
- The fix lands as its own commit, and that commit's subject names its own ticket, in the
  `Area: summary (GTW-N)` style of [git-workflow.md](./git-workflow.md) rule 5.
- A project-manager step of the run files that ticket before the fix is committed, never the build
  agent. `HOUSE_RULES` rule 7 in `.claude/workflows/build-ticket.js` makes those steps the only
  Linear writers, and [linear-discipline.md](./linear-discipline.md) rule 4 names this as the one
  case where the code is written before its ticket exists.
- The build declares the carried fix in its report: the files it touched, the clause the defect
  blocked, and what the defect was.

## Enforcement

`/gate` runs the `design-gate` agent. It audits the diff against the ticket and `docs/` before
any commit. A change that passes the green suite but deviates from spec FAILS the gate. A carried
fix the build declared is not a deviation, so the gate does not fail it as one.
