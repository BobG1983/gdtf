# Design fidelity — build what was specified

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything
> here. It is binding on every word, and it is not optional.**

Why this rule exists: this kit's history includes agents quietly NARROWING a
user-specified design (e.g. "right-sizing" a hard resolution down to something
simpler) because it was easier or faster. That is the number-one failure mode.
Fidelity to the spec beats speed, every time.

## Rules

1. Build EXACTLY what the user / the Linear ticket specifies. Never narrow,
   simplify, or substitute a design because the specified one is harder or
   slower to build.
2. Any deviation — including an apparent improvement — is proposed to the user
   and APPROVED BEFORE building. A silent deviation is a defect: file it with
   `/file-bug`.
3. Linear "Done" is NOT evidence the work exists or matches its spec. Before
   building on top of a "Done" ticket, audit the actual code against the
   ticket text.
4. `docs/` is the design source of truth — it is the contract ALONGSIDE the
   ticket. Design canon: `docs/pillars/`, `docs/combat/`,
   `docs/architecture.md`. When code and docs disagree, stop and surface the
   conflict — never pick a side silently.
5. Ambiguous spec? Ask the user, or route the question through the
   `design-gate` agent. Never resolve ambiguity by choosing the cheapest
   interpretation. Example: a ticket says "the sim resolves hits along an
   arbitrary attack vector" — do NOT collapse it to axis-aligned/grid-snapped
   ECS queries because that is simpler. Build the specified vector math.

## Descoping protocol — the ONLY legal way to shrink scope

- STOP → present the tradeoff to the user → user approves → the TICKET is
  updated FIRST → only then code. Silent descoping is the #1 historical
  failure mode.
- Stop-signal phrases — catching yourself writing one means STOP and ask;
  they are signals, never decisions to make alone: "for now", "simplified
  version", "basic implementation", "we can defer", "out of scope" (applied
  to an in-contract clause), "MVP of this".
- Partial delivery is not delivery: a ticket lands only when EVERY clause is
  implemented. There is no "MVP of a ticket" unless the USER splits it.

## Enforcement

- `/gate` runs the `design-gate` agent: it audits the diff against the ticket
  and `docs/` before any commit. A change that passes the green suite but
  deviates from spec FAILS the gate.
