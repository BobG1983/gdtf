---
name: docs-sync
description: >-
  Re-align docs/ with the code after behavior changes. Walk affected design docs
  claim by claim, verify against Rust source, fix drift, then hand off to /land.
  Use after /gate passes and before /land, or when the docs may be out of date.
argument-hint: "[system or docs/ path]"
---

# /docs-sync: re-align docs/ with the code

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

`.claude/rules/comment-hygiene.md` governs any comment you touch.

## What docs/ may not contain

A reader must be able to act on `docs/` without opening Linear, so nothing in it points there.

No ticket ids. No `GTW-1234`, no ticket slug, no "filed as", no "tracked by". Say what the tree
does today and what is meant to be true. The work that gets it there lives in Linear.

No clause numbers. "clause 15" names a line in a ticket that will be split, renumbered or closed,
so the reference dangles.

No plans, no intentions, no process. Not "we should split this", not "this needs a ticket", not
"TODO", not "on its way out" attached to the name that will do the removing. Write a temporary
state as a state: what the tree carries now, and what replaces it, with no actor.

No dates as promises. A date recording when something was measured is fine. A date implying a
schedule is not.

Measured: `architecture.md` carried `until GTW-1175 moves it (clause 15) and then deletes both
(clause 13)`. GTW-1175 carries `Needs Splitting`, so those clause numbers became children with
different numbers and the sentence pointed at nothing.

The same file used "load-bearing" in four headings, a coined figure of speech that
`plain-language.md` bans. `What breaks without it:` replaced it, the literal thing those sections
say.

## What code decides and what docs/ decides

Code is the authority for what exists. `docs/` is the authority for design intent. If the code is
narrower than the documented design, leave the design text, add a status note, and file a gap
ticket. Never rewrite the design down to match the code (see `design-fidelity.md`).

Evidence rules live in [`.claude/rules/verification.md`](../../rules/verification.md).

## Steps

1. Set the scope from the argument, the recent diff, or recently-Done tickets. Name the changed
   systems.

2. If you are not on a feature branch, create or claim a GTW ticket and branch off develop:
   `git checkout -b feature/gtw-N-docs-sync-<slug>`. Never edit docs on develop or main.

3. Grep `docs/` for the changed names and paths. List the affected docs before editing.

4. Verify every claim against the source and give it a verdict: TRUE / DRIFTED / UNBUILT / RETIRED.
   - TRUE → leave it.
   - DRIFTED → fix the doc text to match the code.
   - UNBUILT → mark it "not yet built" and keep the design.
   - RETIRED → mark it removed, with a one-line reason.

5. Fix in place. Match the doc's voice. Do not rewrite prose that is already right.

6. Hand the doc diff and the verified code files to a read-only design-gate sub-agent. Fix
   anything it rejects.

7. When the docs are clean, point at `/land`. Do not commit between gate and land. `/land` owns
   the commit and is the only writer of `.claude/.gate-pass`, so the pre-commit hook blocks a
   commit made here.
