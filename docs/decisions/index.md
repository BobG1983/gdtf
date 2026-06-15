---
name: Architecture Decision Records
description: Index of GDTF architecture decision records (ADRs) — what they are, the rules, and the current log.
---

# Architecture Decision Records

This directory holds GDTF's **architecture decision records (ADRs)**: one file
per significant, hard-to-reverse decision, capturing the context, the choice,
and its consequences so a future reader can reconstruct *why* without
archaeology. ADRs sit alongside the design canon ([../index.md](../index.md)):
the pillars and combat docs say *what the game is*; ADRs say *what we decided
and why*.

The [`design-notes`](../../.claude/skills/design-notes/SKILL.md) skill writes
new ADRs here from the 0000 template.

## Rules

1. **One decision per file**, numbered `NNNN-kebab-slug.md` starting at `0001`
   (`0000-template.md` is the reserved template, not a real ADR).
2. **Immutable once Accepted.** An Accepted ADR is a historical record — do not
   rewrite its Context or Decision. If reality changes, write a *new* ADR that
   **supersedes** the old one.
3. **Supersede, don't edit.** When a new ADR replaces an old one, set the new
   one's Status to reference what it supersedes, and flip the old one's Status to
   `Superseded by NNNN`. The old file stays; the trail is the value.
4. **Status lifecycle:** `Proposed` → `Accepted` (or → `Superseded`). A
   `Proposed` ADR may still be edited; an `Accepted` one may not.
5. **Factual, not aspirational.** Record the decision that was actually made and
   the reasons that actually drove it. ADRs are evidence, not marketing.
6. **Link, don't duplicate.** Point at the canon (`docs/pillars/`,
   `docs/combat/`, `docs/glossary.md`) and the Linear ticket rather than
   restating them; the ADR captures the *decision*, the canon captures the design.

## The log

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-rust-bevy-rewrite.md) | Reimplement grimdark-turfwar in Rust + Bevy | Accepted |
| [0002](0002-adopt-process-kit.md) | Adopt the `.claude` process kit | Accepted |
| [0003](0003-hand-rolled-data-driven-ui.md) | Hand-rolled, data-driven UI on first-party `bevy_ui` | Accepted |

Template: [0000-template.md](0000-template.md).
