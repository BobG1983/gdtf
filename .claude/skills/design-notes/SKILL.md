---
name: design-notes
description: >-
  Durably capture VERY high-value, long-lived design knowledge into docs/ — the
  game's canon, not a scratchpad. Classifies a note as a math/formula, a design
  pillar, a key decision (ADR), an invariant, or a glossary term, routes it to the
  correct docs/ home with enforced structure (frontmatter, a one-line statement,
  the WHY, source/derivation, pillar + litmus links, glossary/code linkage), and
  adds the index pointer. Use when the user says "note this down", "capture this
  decision", "this is canon", "record this formula/pillar/invariant", "log an ADR",
  or right after a design decision is reached.
when_to_use: >-
  Use to PERMANENTLY record canon: a combat formula and its derivation, a design
  pillar, a key architectural/design decision (as an ADR), an invariant that must
  always hold, or a glossary term that will drive Rust identifiers. NOT for
  transient TODOs, ticket notes, or ephemeral context — this writes the contract
  the rest of the kit measures work against.
argument-hint: "[the note, e.g. \"ADR: square grid for battlescape\" or \"formula: effPen\"]"
allowed-tools: Read, Write, Edit, Grep, Glob
disable-model-invocation: false
user-invocable: true
---

# /design-notes — capture canon into docs/

This kit's failure mode is design knowledge that lives in a chat transcript, a
commit body, or one person's head — then quietly drifts or gets narrowed
(`.claude/rules/design-fidelity.md`). This skill makes a high-value decision
**durable and authoritative** by writing it into `docs/`, the contract ALONGSIDE
the Linear ticket. It is **DOCS-ONLY** — never touch `crates/` or `bins/` here.
Sibling: **/docs-sync** *verifies* existing docs against the code and fixes
drift; **/design-notes** *creates new canon*. Use the right one.

Bar for entry: durable, long-lived, decision-shaping knowledge. If it'd be stale
in a sprint, it's a ticket comment, not canon — decline and say so.

## 1. Classify the note (pick exactly one home)

| Kind | Goes to | Shape |
| --- | --- | --- |
| **MATH / FORMULA** | `docs/combat/<topic>.md` (or the relevant doc) | formula + derivation + worked example + units + code site |
| **DESIGN PILLAR / canon** | `docs/pillars/` | thesis statement + why it's load-bearing + what it forbids |
| **KEY DECISION** | `docs/decisions/NNNN-slug.md` (ADR) | Status / Context / Decision / Consequences / Alternatives |
| **INVARIANT** | the relevant doc (combat/architecture) + glossary if named | the property + how it's enforced/tested |
| **GLOSSARY TERM** | `docs/glossary.md` | the term + meaning + the Rust identifier it drives |

If a note is genuinely two kinds (e.g. a decision that introduces a formula),
the **ADR** is the canonical record and the formula doc links to it — do not
duplicate the prose; cross-link.

## 2. Route to the correct docs/ home

- Start from `docs/index.md` to find the live home; grep `docs/` for the topic so
  you EXTEND an existing section instead of creating a parallel one.
- **ADRs** live under `docs/decisions/`. The number is the **next zero-padded
  4-digit** value: `ls docs/decisions/ | grep -Eo '^[0-9]{4}'` → max + 1 (start at
  `0001` if empty). Filename: `NNNN-<kebab-slug>.md`.
- A glossary term drives **Rust identifiers** — record the exact identifier it
  maps to (a `struct`/`component`/`Resource`/field) so code stays in the house
  vocabulary (no "unit"/"soldier" where "ganger" exists).

## 3. Enforce structure (every note)

Each note carries, in this order:
1. **YAML frontmatter** (see templates) — at minimum `title`, `kind`, `status`,
   `date`, `pillars` (the pillar numbers it serves).
2. **A one-line crisp statement** — the claim in a single bolded sentence.
3. **The WHY** — what's load-bearing; what breaks if it's violated.
4. **Source / derivation** — citation (book/GDC talk/prior art), the math, or the
   discussion it came from. Canon without provenance rots.
5. **Links** — related pillars (`docs/pillars/`), the litmus check, the glossary
   term(s), and the **code site** (`crates/<crate>/src/<module>.rs`) where it
   lives or will live — `**TBD (Bevy):**` if not yet built; never invent a
   module/type/system name to fill the gap.
6. **Litmus check** — answer "does this serve the pillars?" against
   `docs/litmus-tests.md`. A note that fails a litmus is a design smell: surface
   it, don't quietly write it as canon.

Keep it **terse and high-signal** — match the neighboring docs' voice (plain `#`
headings, dense prose, no changelog noise). Canon is read often.

## 4. Write the doc + add the pointer

- Write/extend the doc with the structure above, then add a **one-line pointer to
  `docs/index.md`** under the right section so it's discoverable.
- For an **ADR**, ALSO add a pointer to `docs/decisions/index.md` (create it with
  an `# Decisions` heading + intro line if absent).
- Re-ground engine specifics to Bevy: ECS systems/components/resources,
  `Query`/`Commands`, schedules, `AppState` (`OnEnter`/`OnExit`), `glam`
  `IVec2`/`Vec3`. Never carry Godot terms (node/`.tscn`/`res://`) into new canon.

## 5. Commit path (canon is part of the contract)

A quick capture **writes the doc immediately** and ends there — then OFFER the
follow-up. For a committed canon change, hand off:
- **/gate → /land** — branch off `develop` (`feature/gtw-N-<slug>`), gate the
  docs diff through the read-only `design-gate` review, land as `Docs: <summary>
  (GTW-N)`. Needs a `GTW-N` ticket in project **GDTF** (discover the owning team
  via the Linear MCP — do not hardcode a team name; this skill files a follow-up
  ticket only if asked, hence Linear MCP is otherwise out of its tool scope).
- **/docs-sync** — if the new canon REVEALS that existing docs drifted from the
  code, run docs-sync to reconcile; design-notes only adds, it does not verify
  old claims.

Stage docs **explicitly by name** — never `-A`/`.` (`.claude/rules/git-workflow.md`).

## ADR template (copy verbatim into `docs/decisions/NNNN-slug.md`)

```markdown
---
title: <decision in a noun phrase>
kind: adr
status: Accepted        # Proposed | Accepted | Superseded by NNNN | Deprecated
date: <YYYY-MM-DD>
pillars: [N, ...]       # pillar numbers this decision serves
---

# ADR NNNN: <decision in a noun phrase>

**<One-line statement of what we decided.>**

## Status
Accepted — <date>. (If superseded later, change to "Superseded by ADR NNNN".)

## Context
The forces at play: the problem, constraints, the relevant pillars
(`../pillars/`) and any prior canon. Why a decision is needed now.

## Decision
What we are doing, stated plainly. Name the Rust crate/module/type it lands in
(`crates/<crate>/src/<module>.rs`) or mark **TBD (Bevy):** if not yet built.

## Consequences
What becomes easier and what becomes harder. Invariants this creates; tests or
schedules it implies. Litmus check vs `../litmus-tests.md`.

## Alternatives considered
- **<Option B>** — why rejected.
- **<Option C>** — why rejected.
```

## Formula-note template (copy into `docs/combat/<topic>.md` or the relevant doc)

```markdown
---
title: <formula name>
kind: formula
status: Accepted
date: <YYYY-MM-DD>
pillars: [6, 7]         # e.g. matchups-are-modifiers, balanced-by-construction
---

### <Formula name>

**<One-line statement of what it computes.>**

`<result> = <expression>`   <!-- e.g. effPen = max(0, punch − hardness) -->

- **Units / domain:** <what each term is and its range; e.g. all in px on the
  shared battle-space metric, or integer attribute points>.
- **Derivation / source:** <the math, or the citation — GDC talk, Necromunda
  rule, Paley-wheel construction in two-paradox-tournament.md>.
- **Worked example:** <plug real numbers; show the result> — e.g.
  `punch=4, hardness=1 → effPen = max(0, 3) = 3`.
- **Code site:** `crates/gdtf_battle_sim/src/<module>.rs` (`fn <name>`), or
  **TBD (Bevy):** if not yet built. The sim is render-free and unit-testable.
- **Pillars / litmus:** serves Pillar(s) <N> (`../pillars/`); passes
  `../litmus-tests.md` because <reason>. Glossary terms: <`term`, …>.
```

For an **invariant**, use the formula template's frontmatter with `kind:
invariant`, state the property as the one-liner, and make the **Code site** line
name the test that enforces it (a `#[cfg(test)]` test or integration test under
`crates/<crate>/tests/`). For a **glossary term**, add a row to the
`docs/glossary.md` table: `| **Term** | meaning + the Rust identifier it drives |`.
