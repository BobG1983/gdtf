---
name: docs-sync
description: >-
  Re-align docs/ with the code after behavior changes. Walk affected design docs
  claim by claim, verify against Rust source, fix drift, then hand off to /land.
  Use after /gate passes and before /land, or when docs smell stale.
argument-hint: "[system or docs/ path]"
---

# /docs-sync — re-align docs/ with the code

**`.claude/rules/plain-language.md` governs every word this skill writes.** Read it before writing a ticket, a doc, or a comment.

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

7. **Ready for land.** When clean, point at `/land`. Do not commit yourself between gate and land — that invalidates the fingerprint.
