---
name: next-task
description: >-
  Pick the next Linear ticket and start it the disciplined way: clean tree on
  develop, feature branch, ticket restated as numbered contract, then implement
  and finish via /gate → /docs-sync → /land.
argument-hint: "[GTW-N]"
---

# /next-task: pick and start the next ticket

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

[`.claude/rules/clause-writing.md`](../../rules/clause-writing.md) governs the clauses. Every clause says what changes, where, and what goes red if it is wrong.

## 1. Pick the ticket

With no argument, ask project-manager for the single next ticket in priority order from project GDTF. With the argument `GTW-N`, take that exact ticket.

A `needs-splitting` ticket is never workable. Return it to be split.

## 2. Check for a clean start

Refuse unless `git status --porcelain` is empty and the branch is `develop`.

## 3. Branch

```bash
git checkout develop && git pull origin develop
git checkout -b feature/gtw-N-slug
```

## 4. Move the ticket

Move GTW-N to In Progress through the Linear MCP.

## 5. Restate the ticket as a contract, before any code

Number the clauses C1, C2, and so on. Cover every acceptance criterion and every design-doc reference. Bind each clause to the ticket and to `docs/`. Ask now if anything is ambiguous.

## 6. Audit "Done" dependencies

Audit every Done dependency against the actual code. If broken behavior shipped, file it with `/file-bug`.

## 7. Implement

Build to the contract. Tests must run the real path. Follow [`.claude/rules/verification.md`](../../rules/verification.md). Descoping without the user is illegal.

## 8. Finish

Run `/gate`, then `/docs-sync` if the docs drifted, then `/land`. Only a passing gate makes the ticket eligible for land and Done.
