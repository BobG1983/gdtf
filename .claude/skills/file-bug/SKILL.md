---
name: file-bug
description: >-
  File a defect with root-cause evidence, then fix it through the standard loop:
  investigate to the symbol FIRST, file the Linear bug, THEN branch, failing
  regression test, fix, /gate → /docs-sync → /land.
argument-hint: "[summary]"
---

# /file-bug — root cause first, ticket second, fix third

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything
> here. It is binding on every word, and it is not optional.**

**`.claude/rules/plain-language.md` governs every word this skill writes.** Read it before writing a ticket, a doc, or a comment.

**[`.claude/rules/clause-writing.md`](../../rules/clause-writing.md) governs the clauses.** Every one says what changes, where, and what goes red if it is wrong.

Hard rules:

- **Never fix-then-file.** Ticket exists before the first line of fix code.
- **Never fold a bug fix into an unrelated branch.** One bug = one ticket = one branch.

A silently narrowed design is a defect even if the suite is green (`design-fidelity.md`).

## 1. Investigate root cause FIRST

Reproduce (failing assert or `cargo drun`). Name the **symbol** and quote the line — a bare line number rots, see `linear-discipline.md`. Find the shipped ticket via `git log -S` / `git blame`. If narrowed design, cite the `docs/` source.

## 2. File the Linear bug

Via project-manager, create in project GDTF with four sections:

- **Shipped claim vs actual**
- **Root cause** (the symbol, and the line quoted)
- **Fix contract** (clause-numbered C1, C2, …)
- **Test plan** (regression tests that fail before, pass after, real path)

## 3. THEN branch

Clean tree on `develop`. `git checkout -b feature/gtw-N-slug` off develop. Move to In Progress.

## 4. Implement

Write the failing regression test first. Fix. Watch it pass. Follow [`.claude/rules/verification.md`](../../rules/verification.md).

## 5. Gate → docs-sync → land

`/gate` against the fix contract. `/docs-sync` if docs drifted. `/land`.
