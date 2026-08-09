---
name: file-bug
description: >-
  File a defect with root-cause evidence, then fix it through the standard loop:
  investigate to file:line FIRST, file the Linear bug, THEN branch, failing
  regression test, fix, /gate → /docs-sync → /land.
argument-hint: "[summary]"
---

# /file-bug — root cause first, ticket second, fix third

**`.claude/rules/plain-language.md` governs every word this skill writes.** Read it before writing a ticket, a doc, or a comment.

Hard rules:

- **Never fix-then-file.** Ticket exists before the first line of fix code.
- **Never fold a bug fix into an unrelated branch.** One bug = one ticket = one branch.

A silently narrowed design is a defect even if the suite is green (`design-fidelity.md`).

## 1. Investigate root cause FIRST

Reproduce (failing assert or `cargo drun`). Cite **file:line**. Find the shipped ticket via `git log -S` / `git blame`. If narrowed design, cite the `docs/` source.

## 2. File the Linear bug

Via project-manager, create in project GDTF with four sections:

- **Shipped claim vs actual**
- **Root cause** (file:line)
- **Fix contract** (clause-numbered C1, C2, …)
- **Test plan** (regression tests that fail before, pass after, real path)

## 3. THEN branch

Clean tree on `develop`. `git checkout -b feature/gtw-N-slug` off develop. Move to In Progress.

## 4. Implement

Write the failing regression test first. Fix. Watch it pass. Follow [`.claude/rules/verification.md`](../../rules/verification.md).

## 5. Gate → docs-sync → land

`/gate` against the fix contract. `/docs-sync` if docs drifted. `/land`.
