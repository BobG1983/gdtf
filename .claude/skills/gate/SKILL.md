---
name: gate
description: >-
  Verify an implementation against its ticket contract before landing. Pulls the
  Linear ticket(s), restates them as a clause-numbered contract, runs the suite, and has
  a design-gate sub-agent verify the diff first-hand. Use after implementing a
  ticket (or a runtime-coupled set that must land together) and before /land.
argument-hint: "[GTW-N ...]"
---

# /gate — contract check before landing

Why this exists: agents quietly **narrow** user-specified designs to "easy over
right" and claim completion without running anything. The gate freezes the ticket as
a contract and has an independent design-gate sub-agent check the code against it.
Binding background: `.claude/rules/design-fidelity.md`, `.claude/rules/verification.md`,
and the design canon under `docs/` (pillars, combat, glossary, litmus-tests, ADRs) —
that canon is part of the contract ALONGSIDE the ticket.

## Steps

1. **Resolve the ticket(s).** The `GTW-N` argument may name a single ticket OR a
   space/comma-separated SET of tickets (for runtime-coupled work — e.g. GTW-2 + GTW-3
   — that must land together); if absent, derive the single ticket from the branch name
   (`feature/gtw-N-slug`) and confirm with the user. Single-ticket is the common path.
   Pull the FULL ticket for EACH named id via the Linear MCP — project **GDTF**; discover
   the owning team through the MCP (the team that owns project GDTF), do NOT hardcode a
   team name. Read title, description, acceptance criteria, comments. Never gate from
   memory of the ticket.
2. **Run the suite — green is mandatory at gate time.** The one definition of green,
   from the repo root (`$CLAUDE_PROJECT_DIR`):

   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings
   cargo test --workspace --features grimdark_turfwar/dynamic_linking
   ```

   (`cargo dclippy` / `cargo dtest` are the shorthand; aliases in `.cargo/config.toml`.)
   Green = all three exit 0. The workspace `Cargo.toml` denies clippy
   all/pedantic/correctness plus unwrap/expect/panic/todo/unimplemented and
   missing_docs, so fmt-clean and lint-clean ARE part of green. Red → the gate FAILS
   immediately; fix the suite before anything else.
   - Note: gdtf has few or zero tests today, so `cargo test` may pass trivially. That
     is NOT a free pass — a behavioral ticket whose clauses add no test on the real
     code path is a contract VIOLATION (see step 7), not a green light.
3. **Restate the contract as clause-numbered** (C1, C2, …): one clause per
   requirement / acceptance criterion, faithful to the ticket's wording and to any
   `docs/` canon it invokes — do not soften, merge, or drop clauses. When MULTIPLE
   tickets were named, restate ALL of their contracts (the combined clause set covers
   every named ticket); attribute each clause to its source ticket. Sim behavior
   (gdtf_battle_sim) clauses must name the observable outcome and the test that proves
   it. From here the contract is **read-only**; it is what the code is measured
   against, not a draft to negotiate with.
4. **Gather the diff.** Capture, including untracked files:

   ```bash
   git status
   git diff develop...HEAD
   git ls-files -o --exclude-standard      # untracked, so the gate sees new files
   ```

   (Use `git diff develop...HEAD` when commits exist on the branch; otherwise
   `git diff HEAD`. Untracked files are part of the change and must be reviewed.)
5. **Verify the diff first-hand via the design-gate workflow.** This is a workflow
   orchestration step, not a standing team: the orchestrating session spawns a
   per-need **design-gate** sub-agent and passes it the ticket id(s), the numbered
   contract, the `docs/` canon paths it touches, the diff, and this instruction
   verbatim: "Verify every clause FIRST-HAND — read the code and run what you must
   (including `cargo run -p grimdark_turfwar --features dynamic_linking` or a headless
   Bevy integration test for runtime behavior); do not trust the implementer's claims or
   this summary." When MULTIPLE tickets were named, the design-gate audits the ONE
   combined diff against the combined contract (every clause of every named ticket) in a
   single pass. The
   sub-agent reports back to the spawning session; it does not spawn further
   sub-agents.
6. **Relay the verdict** to the user: per-clause PASS / VIOLATION with the sub-agent's
   evidence. Do not editorialize a VIOLATION into a pass.
7. **On violations, repair the CODE.** Fix each violated clause in the implementation
   (Rust crates under `crates/`, the binary under `bins/`). **Forbidden:** editing the
   contract, deleting or weakening acceptance criteria, or narrowing the design so the
   check passes — that is the exact failure this gate exists to stop. If you believe a
   clause itself is wrong, STOP and ask the user; never self-amend the contract. If
   fixing a violation properly looks **"too expensive"**, that is a USER decision —
   present the tradeoff and STOP; never engineer around it, never quietly re-scope.
8. **Re-gate after repairs**: repeat steps 2–6 against the SAME contract. **Maximum 2
   repair rounds.** If violations remain after the second re-gate, STOP — report the
   remaining violations to the user, leave the ticket open, and do not proceed to /land.
9. **On full PASS, record it for /land** — write ONE `.claude/.gate-pass` (gitignored via
   `.claude/.gitignore`, so writing it cannot disturb the fingerprint it records;
   never stage it). With multiple tickets, the single `TICKET=` line lists them all,
   comma-separated (e.g. `TICKET=GTW-2,GTW-3`):

   ```bash
   { printf 'TICKET=%s\n' "GTW-N"        # comma-separated for a multi-ticket set
     printf 'BRANCH=%s\n' "$(git branch --show-current)"
     printf 'HEAD=%s\n' "$(git rev-parse HEAD)"
     printf 'FINGERPRINT=%s\n' "$({ git rev-parse HEAD; git status --porcelain; git diff HEAD; git ls-files -o --exclude-standard -z | LC_ALL=C sort -z | xargs -0 -r shasum -a 256; } | shasum -a 256 | cut -d' ' -f1)"
   } > .claude/.gate-pass
   ```

   The fingerprint covers HEAD, all tracked changes, AND the **content of every
   untracked file** (`git ls-files -o`) — an all-untracked tree is fully covered.
   The pre-commit hook reads this file too: without it (or with a stale BRANCH/HEAD)
   every `git commit` is blocked. The hook and `.claude/.gitignore` are this skill's
   companions — `/gate` writes the fingerprint, the hook enforces it, `/land` consumes
   and clears it.
10. **Move every named ticket to In Review** via the Linear MCP (project **GDTF**, team
    discovered via MCP) — for a multi-ticket set, move ALL of them — statuses move with
    the work (`.claude/rules/linear-discipline.md`). Then report PASS and point the user
    at `/land`. /gate NEVER moves a ticket to Done — only landing does.
