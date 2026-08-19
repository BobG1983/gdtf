---
name: docs-sync
description: >-
  Re-align docs/ with the code after behavior changes. Walk affected design docs
  claim by claim, verify against Rust source, fix drift, then hand off to /land.
  Use after /gate passes and before /land, or when docs smell stale.
argument-hint: "[system or docs/ path]"
---

# /docs-sync — re-align docs/ with the code

**`.claude/rules/plain-language.md` governs every word this skill writes.** Read it before writing a ticket, a doc, or a comment. `.claude/rules/comment-hygiene.md` governs any comment you touch in the same pass.

## A doc says what is true, never who is going to change it

`docs/` is design canon. A reader must be able to act on it without opening the board, so it
carries no pointer to the board at all:

- **No ticket ids.** No `GTW-1234`, no ticket slug, no "filed as", no "tracked by". Say what the
  tree does today and what is meant to be true; the work that gets it there lives in Linear.
- **No clause numbers.** "clause 15" names a line in a ticket that will be split, renumbered or
  closed, and the reference dangles the moment that happens.
- **No plans, no intentions, no process.** Not "we should split this", not "this needs a ticket",
  not "TODO", not "on its way out" attached to a name that will do the removing. A temporary state
  is written as the state — what the tree carries, and what replaces it — with no actor.
- **No dates as promises.** A date recording when something was measured is fine. A date implying
  a schedule is not.

Measured: `architecture.md` carried `until GTW-1175 moves it (clause 15) and then deletes both
(clause 13)`. GTW-1175 carries `Needs Splitting`, so those clause numbers became children with
different numbers and the sentence pointed at nothing.

The same file used **load-bearing** in four headings, which `plain-language.md` names as a coined
figure of speech. `What breaks without it:` replaced it — the literal thing the section says.

**Code is authority for what exists.** **docs/ remain authority for design intent.**
If code is narrower than the documented design, leave the design text, add a status note, and file a gap ticket. Never quietly rewrite the design down to match the code (see `design-fidelity.md`).

Evidence rules: [`.claude/rules/verification.md`](../../rules/verification.md).

## Steps

1. **Scope.** Argument, recent diff, or recently-Done tickets. Name the changed systems.

2. **Ticket + branch** (if not already on a feature branch). Create/claim a GTW ticket if needed; `git checkout -b feature/gtw-N-docs-sync-<slug>` off develop. Never edit docs on develop/main.

3. **Map the docs.** Grep `docs/` for the changed names/paths. List affected docs before editing.

4. **Verify claim by claim** against the actual source. Verdict each: TRUE / DRIFTED / UNBUILT / RETIRED.
   - TRUE → leave.
   - DRIFTED → fix the doc text to the code.
   - UNBUILT → mark "not yet built"; keep the design.
   - RETIRED → tombstone with one-line why.

5. **Fix in place.** Match the doc's existing voice. Do not rewrite healthy prose.

6. **Design-gate review.** Hand the doc diff + verified code files to a read-only design-gate sub-agent. Fix anything it rejects.

7. **Ready for land.** When clean, point at `/land`. Do not commit yourself between gate and land — `/land` owns the commit, and it is the only writer of `.claude/.gate-pass`, so a commit here has no gate-pass and the pre-commit hook blocks it.
