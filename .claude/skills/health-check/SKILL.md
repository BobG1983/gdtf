---
name: health-check
description: >-
  Periodic repo health sweep for dead code, real bugs and test gaps, run as
  parallel read-only sub-agents. A finding becomes a ticket or a change only
  after it survives an adversarial refutation pass.
argument-hint: "[optional scope, e.g. crates/gdtf_battle_sim]"
---

# /health-check

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Evidence rules: [`.claude/rules/verification.md`](../../rules/verification.md). Design contract: `docs/` + `design-fidelity.md`.

## Hard rules

Do not file a ticket, delete code, or fix anything until an independent refutation pass confirms
the finding. If the refuter is unsure, the finding is refuted and dropped.

The sweep and refutation sub-agents are read-only.

This skill files tickets. It does not fix. Fixes go through `/next-task` → `/gate` → `/docs-sync` →
`/land`, one ticket per branch.

## Steps

1. Use the argument as the scope, if given. Otherwise sweep `crates/` and `bins/`, skipping
   `target/`.

2. Run the sweep as three sub-agents in parallel, one per lens.

   Dead code: call a symbol dead only after a full reference sweep: compiler warnings,
   cargo machete/udeps, name greps, and how Bevy reaches code indirectly (`add_systems`,
   `add_plugins`, `init_resource`, `register_type`, `run_if`, asset and string paths). If any check
   hits, or was skipped, the symbol is not dead.

   Bugs: trace real scenarios end to end. Name the symbol, quote the line, and give a concrete
   scenario. No smells, no style notes.

   Test gaps: grade each public item COVERED, WEAK or UNCOVERED. Happy path only is WEAK. Report
   the highest-risk gaps first.

3. Hand every finding to an independent adversarial sub-agent: "REFUTE this. Hunt hidden
   references, guards, existing tests." It answers CONFIRMED or REFUTED with evidence.

4. For confirmed dead code only, delete what is accidental or superseded. Keep code that `docs/`
   says is deliberately dormant, and file a ticket for a comment saying so instead.

5. Bugs go through `/file-bug`. Removing dead code, adding a dormancy comment and filling a test
   gap go through project-manager (project GDTF). Put the evidence and the refuter's verdict in the
   body.

6. Never batch unrelated findings into one ticket.

7. Report per lens: what was swept, refuted and confirmed, and which tickets were filed. List the
   deliberately dormant items you left alone, and why.
