---
name: design-notes
description: >-
  Capture VERY high-value, long-lived design knowledge into docs/, the game's
  canon. Classifies a note as a math/formula, a design pillar, an invariant, or a
  glossary term, routes it to the correct docs/ home with enforced structure
  (frontmatter, a one-line statement, the WHY, source/derivation, pillar + litmus
  links, glossary/code linkage), and adds the index pointer. Use when the user
  says "note this down", "capture this decision", "this is canon", "record this
  formula/pillar/invariant", or right after a design decision is reached.
when_to_use: >-
  Use to PERMANENTLY record canon: a combat formula and its derivation, a design
  pillar, an invariant that must always hold, or a glossary term that will drive
  Rust identifiers. NOT for transient TODOs, ticket notes, or ephemeral context.
  This writes the contract the rest of the kit measures work against.
argument-hint: "[the note, e.g. \"invariant: the sim never reads the presenter\" or \"formula: effPen\"]"
allowed-tools: Read, Write, Edit, Grep, Glob
disable-model-invocation: false
user-invocable: true
---

# /design-notes: capture canon into docs/

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Design knowledge that never reaches `docs/` drifts or gets narrowed
(`.claude/rules/design-fidelity.md`). Write the decision into `docs/`, the contract
ALONGSIDE the Linear ticket. Never touch `crates/` or `bins/` here.

Bar for entry: long-lived, decision-shaping knowledge. If it would be stale in a sprint,
decline and say it belongs in a ticket comment.

## 1. Classify the note (pick exactly one home)

| Kind | Goes to | Shape |
| --- | --- | --- |
| **MATH / FORMULA** | `docs/combat/<topic>.md` (or the relevant doc) | formula + derivation + worked example + units + code site |
| **DESIGN PILLAR / canon** | `docs/pillars/` | thesis statement + what depends on it + what it forbids |
| **INVARIANT** | the relevant doc (combat/architecture) + glossary if named | the property + how it's enforced/tested |
| **GLOSSARY TERM** | `docs/glossary.md` | the term + meaning + the Rust identifier it drives |

If a note is two kinds at once, for example an invariant that rests on a formula, the
formula doc is the record and the invariant links to it. Do not duplicate the prose.

## 2. Route to the correct docs/ home

Start from `docs/index.md`. Grep `docs/` for the topic so you EXTEND an existing
section instead of creating a parallel one.

Record the exact Rust identifier a glossary term drives (a
`struct`/`component`/`Resource`/field) so code uses the project's own words. No "unit" or
"soldier" where "ganger" exists.

## 3. Enforce structure

Each note carries these six things, in this order.

1. YAML frontmatter, as in the templates below. At minimum `title`, `kind`, `status`,
   `date`, and `pillars` (the pillar numbers it serves).
2. The claim, in a single bolded sentence.
3. The why. Say what breaks if the note is violated.
4. Source or derivation. Give the citation (book, GDC talk, prior art), the math, or the
   discussion it came from. Canon with no source cannot be checked later.
5. Links to the related pillars (`docs/pillars/`), the litmus check, the glossary terms,
   and the code site (`crates/<crate>/src/<module>.rs`) where it lives. Write
   `**TBD (Bevy):**` if it is not yet built. Never invent a module, type, or system name to
   fill the gap.
6. Litmus check. Answer "does this serve the pillars?" against `docs/litmus-tests.md`. A note
   that fails a litmus is a sign the design is wrong. Say so instead of quietly writing it as
   canon.

Keep the note short and specific. Match the neighbouring docs' voice: plain `#` headings,
dense prose, no changelog noise.

## 4. Write the doc and add the pointer

Write or extend the doc using the structure above. Then add a one-line pointer to
`docs/index.md` under the right section.

Describe engine specifics in Bevy terms only: entity, component, system, resource, asset,
`Query`/`Commands`, schedules, `AppState` (`OnEnter`/`OnExit`), and `glam` `IVec2`/`Vec3`.

## 5. Commit path

A quick capture writes the doc immediately and ends there. Then OFFER the follow-up.

For a committed canon change, branch off `develop` (`feature/gtw-N-<slug>`), run /gate to
put the docs diff through the read-only `design-gate` review, then /land it as
`Docs: <summary> (GTW-N)`. That needs a `GTW-N` ticket in project **GDTF**.
Discover the owning team through the Linear MCP, never a hardcoded name. This skill files a
follow-up ticket only if asked.

This skill only adds canon. /docs-sync verifies existing docs against the code and fixes
drift, so run it if the new canon reveals that a doc drifted.

Stage docs explicitly by name. Never `-A` or `.` (`.claude/rules/git-workflow.md`).

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

- Units / domain: <what each term is and its range; e.g. all in px on the
  shared battle-space metric, or integer attribute points>.
- Derivation / source: <the math, or the citation: a GDC talk, a Necromunda
  rule, the Paley-wheel construction in two-paradox-tournament.md>.
- Worked example: <plug real numbers; show the result>, e.g.
  `punch=4, hardness=1 → effPen = max(0, 3) = 3`.
- Code site: `crates/gdtf_battle_sim/src/<module>.rs` (`fn <name>`), or
  **TBD (Bevy):** if not yet built. The sim is render-free and unit-testable.
- Pillars / litmus: serves Pillar(s) <N> (`../pillars/`); passes
  `../litmus-tests.md` because <reason>. Glossary terms: <`term`, …>.
```

For an invariant, use the formula template's frontmatter with `kind: invariant`, state the
property as the one-liner, and make the Code site line name the test that enforces it: a
`#[cfg(test)]` test, or an integration test under `crates/<crate>/tests/`.

For a glossary term, add a row to the `docs/glossary.md` table:
`| **Term** | meaning + the Rust identifier it drives |`.
