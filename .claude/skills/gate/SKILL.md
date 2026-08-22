---
name: gate
description: >-
  Verify an implementation against its ticket contract before landing. Pulls the
  Linear ticket(s), restates them as a clause-numbered contract, runs the full
  green suite, and has design-gate sub-agents verify the diff. Use after
  implementing a ticket and before /docs-sync and /land.
argument-hint: "[GTW-N ...]"
---

# /gate (contract check before landing)

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Also binding: `.claude/rules/design-fidelity.md`, `.claude/rules/verification.md`, and `docs/`.

## Suite scope (docs-only skip)

Decide the scope from the changed paths: staged, committed on the branch against `develop`, and untracked. Never decide it from the ticket title. Record `SCOPE=FULL` or `SCOPE=DOCS` in the gate report. `/land` decides the scope again for the tree it commits, and writes the `SCOPE=` line into `.claude/.gate-pass`.

The default is FULL.

The scope is DOCS only if every changed path is a `*.md` file under one of:

- `docs/`
- `.claude/`
- the repo root (e.g. `CLAUDE.md`, `README.md`)

The scope is FULL, even when every changed path is markdown, if the set includes `.claude/rules/verification.md`. The scope rule itself lives in that file.

The scope is also FULL for mixed diffs, renames that touch a non-markdown path, empty path sets, and anything you cannot classify.

Under DOCS, do not run cargo. Say which paths you read the scope from.

Under FULL, run the full suite from [`.claude/rules/verification.md`](../../rules/verification.md). Every command it lists must exit 0. Use the aliases. A red command fails the gate at once.

Pre-commit does not implement this skip. It always runs its cargo subset.

## Steps

1. Resolve the tickets from the argument or the branch name (`feature/gtw-N-slug`). Pull each full ticket through the Linear MCP (project GDTF). Never gate from memory.

2. Apply **Suite scope** above.

3. Restate the contract with numbered clauses (C1, C2, …). One clause per requirement, faithful to the ticket and to any `docs/` it invokes. With several tickets, combine the clauses and attribute each one. The contract is then read-only.

4. Gather the diff, including untracked files:

   ```bash
   git status
   git diff develop...HEAD
   git ls-files -o --exclude-standard
   ```

5. Run the blocking checks. A failure here fails the gate, like a clause violation.
   - 4a Tests. Every behavioral clause has a test that runs the real path, carries an assertion, and would go red if the behavior were wrong (`verification.md` rules 2–3).
   - 4b Wiring. Systems, plugins and resources the ticket claims run are actually registered.
   - 4c Size. Warn above 300 lines, block above 400, unless the file is cohesive and the ticket sanctioned it.
   - 4d Hygiene. No `GTW-` strings, no banned jargon, short doc comments.

6. Spawn the read-only design-gate sub-agents in parallel, one per lens: clauses, tests, rules. Give each one the contract and the diff, and tell it to verify first-hand. Any non-compliant lens blocks the gate.

7. Relay the verdict: PASS or VIOLATION per clause, with the evidence.

8. On a violation, repair the code, not the contract. Two repair rounds at most, then stop and report.

9. On a full pass, report it with the `SCOPE=` line and the ticket. Move the tickets to In Review through the Linear MCP. Point at `/docs-sync`, then `/land`. **Do not write `.claude/.gate-pass`.** `/land` writes it, from the tree it is about to commit. `/docs-sync` runs between the two and moves that tree, so a file written here would describe a tree nobody commits.
