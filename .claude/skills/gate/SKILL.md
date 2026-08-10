---
name: gate
description: >-
  Verify an implementation against its ticket contract before landing. Pulls the
  Linear ticket(s), restates them as a clause-numbered contract, runs the full
  green suite, and has design-gate sub-agents verify the diff. Use after
  implementing a ticket and before /docs-sync and /land.
argument-hint: "[GTW-N ...]"
---

# /gate — contract check before landing

Binding background: `.claude/rules/design-fidelity.md`, `.claude/rules/verification.md`, and `docs/`.

## Suite scope (docs-only skip)

Decide scope from **changed paths** (staged, committed on the branch vs `develop`, and untracked). Not from the ticket title. Record `SCOPE=FULL` or `SCOPE=DOCS` in the gate report and in `.claude/.gate-pass`.

**Default is FULL.** When unsure, FULL.

**DOCS** only if every changed path is a `*.md` file under one of:

- `docs/`
- `.claude/`
- the repo root (e.g. `CLAUDE.md`, `README.md`)

**Always FULL** (even if markdown) when the set includes:

- `docs/tooling/qa-commands.md`
- `.claude/rules/verification.md`

**Always FULL** for mixed diffs, renames that touch a non-markdown path, empty path sets, or anything you cannot classify.

**DOCS means:** do not run cargo. State that scope was DOCS and which paths you used.

**FULL means:** run the full suite from [`.claude/rules/verification.md`](../../rules/verification.md). All eight must exit 0. Use the aliases. Red → fail immediately.

Pre-commit does not implement this skip — it always runs its cargo subset. Scope is agent judgment in this skill and in `/land`.

## Steps

1. **Resolve the ticket(s).** Argument or branch name (`feature/gtw-N-slug`). Pull full ticket(s) via Linear MCP (project GDTF). Never gate from memory.

2. **Scope, then suite.** Apply **Suite scope** above. Run cargo only when FULL.

3. **Restate the contract as clause-numbered** (C1, C2, …). One clause per requirement. Faithful to the ticket and any `docs/` it invokes. Multi-ticket: combine and attribute. Contract is then read-only.

4. **Gather the diff** (including untracked):

   ```bash
   git status
   git diff develop...HEAD
   git ls-files -o --exclude-standard
   ```

5. **Blocking checks** (fail the gate like a clause violation):
   - **4a Tests** — every behavioral clause has a real-path, assertion-bearing, pin-discriminating test (`verification.md` rules 2–3).
   - **4b Wiring** — systems/plugins/resources that the ticket claims run are actually registered.
   - **4c Size** — warn >300 / block >400 lines unless cohesive and ticket-sanctioned.
   - **4d Hygiene** — no `GTW-` strings, no banned jargon, short doc comments.

6. **Design-gate fan-out.** Spawn three parallel read-only design-gate sub-agents (fidelity / tests / structure+Bevy). Pass the contract, diff, and instruction to verify first-hand. Merge any-non-compliant-blocks.

7. **Relay the verdict.** Per-clause PASS / VIOLATION with evidence.

8. **On violations, repair the code** (not the contract). Max 2 repair rounds, then stop and report.

9. **On full PASS, write `.claude/.gate-pass`** (TICKET, BRANCH, HEAD, **SCOPE**). Move ticket(s) to In Review via Linear MCP. Point at `/docs-sync` then `/land`.
