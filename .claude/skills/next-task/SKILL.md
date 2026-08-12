---
name: next-task
description: >-
  Pick the next Linear ticket and start it the disciplined way — clean tree on
  develop, feature branch, ticket restated as numbered contract,
  then implement and finish via /gate → /docs-sync → /land.
argument-hint: "[GTW-N]"
---

# /next-task — start the next ticket the disciplined way

**`.claude/rules/plain-language.md` governs every word this skill writes.** Read it before writing a ticket, a doc, or a comment.

**[`.claude/rules/clause-writing.md`](../../rules/clause-writing.md) governs the clauses.** Every one says what changes, where, and what goes red if it is wrong.

## 1. Pick the ticket

- No argument: ask project-manager for the ONE next ticket in priority order (project GDTF).
- Argument `GTW-N`: that exact ticket.
- A `needs-splitting` ticket is never workable — return it for decomposition.

## 2. Verify a clean start

`git status --porcelain` empty **and** on `develop`. Otherwise refuse.

## 3. Branch

```bash
git checkout develop && git pull origin develop
git checkout -b feature/gtw-N-slug
```

## 4. Move the ticket

Move GTW-N to **In Progress** via Linear MCP.

## 5. Restate as contract — BEFORE any code

Clause-numbered (C1, C2, …). Every acceptance criterion and design-doc reference. Bound to the ticket **and** `docs/`. Ask now if ambiguous.

## 6. Audit "Done" dependencies

For every Done dependency: audit against the actual code. File with `/file-bug` if broken behavior shipped.

## 7. Implement

Build to the contract. Real-path tests required. Follow [`.claude/rules/verification.md`](../../rules/verification.md). Descoping is illegal without the user.

## 8. Finish

`/gate` → `/docs-sync` (if docs drifted) → `/land`. Only a passing gate makes the ticket eligible for land and Done.
