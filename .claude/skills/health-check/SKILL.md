---
name: health-check
description: >-
  Periodic repo health sweep — dead code, real bugs, test gaps — fanned out as
  parallel read-only sub-agents, with mandatory adversarial refutation before
  any finding becomes a ticket or change.
argument-hint: "[optional scope, e.g. crates/gdtf_battle_sim]"
---

# /health-check — dead code / bugs / test gaps, adversarially verified

Evidence rules: [`.claude/rules/verification.md`](../../rules/verification.md). Design contract: `docs/` + `design-fidelity.md`.

## Hard rules

- **Refute-before-act:** no ticket, deletion, or fix until an independent refutation pass confirms the finding. Uncertain = refuted = dropped.
- Sweep and refutation sub-agents are **read-only**.
- This skill files tickets; it does not fix. Fixes go through `/next-task` → `/gate` → `/docs-sync` → `/land`, one ticket per branch.

## Steps

1. **Scope.** Argument or full `crates/` + `bins/` (skip `target/`).

2. **Fan out the sweep** — three parallel read-only sub-agents:
   - **Dead code.** Only after full reference sweep (compiler warnings, cargo machete/udeps, name greps, Bevy indirection: `add_systems`/`add_plugins`/`init_resource`/`register_type`/`run_if`, asset/string paths). A hit or skipped check = not dead.
   - **Bugs.** Trace real scenarios end-to-end. Name the symbol, quote the line, give a concrete scenario. No smells or style notes.
   - **Test gaps.** Grade public surface COVERED / WEAK / UNCOVERED. Happy-path-only = WEAK. Highest-risk first.

3. **Refutation pass (mandatory).** For every finding, independent adversarial sub-agent: "REFUTE this. Hunt hidden references, guards, existing tests." CONFIRMED or REFUTED with evidence. Uncertain = REFUTED.

4. **Keep-vs-delete (confirmed dead only).** Delete accidental/superseded. KEEP designed-dormant surface specified in `docs/` — ticket a dormancy comment instead.

5. **File tickets.** Bugs via `/file-bug`. Dead-code removals, dormancy comments, test-gap work via project-manager (project GDTF). Put evidence + refuter verdict in the body.

6. **Fix later via the standard loop.** `/next-task` → implement → `/gate` → `/docs-sync` → `/land`. Never batch unrelated findings.

7. **Report.** Per lens: swept / refuted / confirmed / tickets filed, plus designed-dormant items left alone and why.
