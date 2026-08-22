---
name: file-bug
description: >-
  File a defect with root-cause evidence, then fix it through the standard loop:
  investigate to the symbol FIRST, file the Linear bug, THEN branch, failing
  regression test, fix, /gate → /docs-sync → /land.
argument-hint: "[summary]"
---

# /file-bug

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

[`.claude/rules/clause-writing.md`](../../rules/clause-writing.md) governs the clauses. Each one says what changes, where, and what goes red if it is wrong.

## Hard rules

The ticket exists before the first line of fix code.

Each bug gets its own ticket and branch.

A silently narrowed design is a defect even when the suite is green (`design-fidelity.md`).

## 1. Investigate the root cause

Reproduce it, with a failing assert or `cargo drun`. Name the symbol and quote the line. A bare line number rots, see `linear-discipline.md`. Find the ticket that shipped the bug with `git log -S` or `git blame`. If the design was narrowed, cite the `docs/` source.

## 2. File the Linear bug

Have project-manager create it in project GDTF with four sections:

- Shipped claim vs actual
- Root cause (the symbol, and the line quoted)
- Fix contract (clauses numbered C1, C2, …)
- Test plan (regression tests that fail before, pass after, real path)

## 3. Branch

From a clean tree on `develop`, run `git checkout -b feature/gtw-N-slug`. Move the ticket to In Progress.

## 4. Implement

Write the failing regression test first. Make the fix. Watch the test pass. Follow [`.claude/rules/verification.md`](../../rules/verification.md).

## 5. Gate, docs-sync, land

Run `/gate` against the fix contract. Run `/docs-sync` if the docs drifted. Then `/land`.
