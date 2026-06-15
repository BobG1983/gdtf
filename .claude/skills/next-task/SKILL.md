---
name: next-task
description: >-
  Pick the next Linear ticket and start it the disciplined way — PM-chosen
  priority (never a LARGE item), clean tree on develop, git flow feature
  branch, the ticket restated as a numbered contract, "Done" dependencies
  audited against the code, tests on the real path, finished via /gate. Use
  when the user says "next task", "what's next", "pick up a ticket", or
  "start GTW-N".
argument-hint: "[GTW-N]"
---

# /next-task — start the next ticket the disciplined way

Why this exists: this project's lineage includes quietly narrowed designs,
tickets marked Done that weren't, completion claimed without running anything,
and giant multi-ticket uncommitted trees. Each step below closes one of those
holes. Do them **in order**; skip nothing.

This is a Rust + Bevy 0.18 ECS workspace (`crates/*`, `bins/*`). The
orchestrating workflow / main session invokes the sub-agents named below
per-need and relays their findings to the user — there is no persistent team.

## 1. Pick the ticket

- No argument: invoke the **project-manager** sub-agent (Agent tool) and ask
  for the ONE next ticket in priority order from Linear, project **GDTF** —
  full description, labels, and dependencies. The PM discovers the owning
  Linear team via the Linear MCP (the team that owns project GDTF); do not
  hardcode a team name.
- Argument `GTW-N` given: ask the project-manager sub-agent for that exact
  ticket instead. The argument overrides priority order, **not** the rest of
  this procedure.
- **A `needs splitting`-labelled ticket is never workable.** If the pick carries
  `needs-splitting`, stop and return it to the user for decomposition into sub-issues
  (the project-manager sub-agent creates the children once the user agrees the
  split). Do not start it, and do not "just do part of it".

## 2. Verify a clean start — refuse otherwise

Run `git status --porcelain` and `git branch --show-current`. Require BOTH:

- working tree clean (no modified, staged, or untracked source files), AND
- current branch is `develop`.

If either fails, **refuse to start**: tell the user to finish or stash the
in-flight work first. Never stack a new ticket onto a dirty tree — that is
exactly how multi-ticket monster trees happened in the lineage. See
`.claude/rules/git-workflow.md`.

## 3. Branch

`git flow feature start gtw-N-slug` (lowercase ticket id + short kebab slug,
e.g. `gtw-164-next-task-skill`) → `feature/gtw-N-slug` off `develop`.

## 4. Move the ticket

Via the project-manager sub-agent: move GTW-N to **In Progress**. The board
must mirror reality at every moment — see `.claude/rules/linear-discipline.md`.

## 5. Restate the ticket as a contract — BEFORE any code

In the conversation, restate the ticket as a **clause-numbered contract**
(C1, C2, …): every acceptance criterion, constraint, and design-doc reference
as its own numbered clause. This is the anti-narrowing device — /gate checks
the work against these clauses, and a clause you never wrote down is a clause
you will silently drop. The contract is bound jointly to the Linear ticket AND
the design canon in `docs/` (pillars, combat specs, glossary, litmus-tests,
ADRs, architecture). If a clause is ambiguous, ask the user NOW, not
mid-implementation. Never trade a clause away for "easy/fast over right".

## 6. Audit "Done" dependencies

For every ticket this one depends on that is marked Done: audit the claim
against the actual code per `.claude/rules/design-fidelity.md` before building
on it (you may delegate the audit to the **design-gate** sub-agent via the
Agent tool). Done labels have been wrong in this lineage before. If a
dependency is not actually done, stop, tell the user, and file it with
**/file-bug** if broken behavior shipped.

## 7. Implement

Build to the contract. Tests must exercise the **real code path** — the same
systems, components, and resources the app runs, never a reimplementation of
the logic inside the test. New combat/sim logic belongs in the render-free
authoritative sim crate `gdtf_battle_sim` (the MODEL) and is unit-testable
with injected seeded RNG; the presenter (`gdtf_battle_presenter`, the VIEW)
mirrors it. Place tests as in-crate `#[cfg(test)] mod tests` or integration
tests under `crates/<crate>/tests/` — NOT `res://test`, NOT GUT.

Follow `.claude/rules/verification.md` and `.claude/rules/bevy-traps.md` (ECS
gotchas: system ordering / schedule placement, `OnEnter`/`OnExit` AppState
transitions, Query filter conflicts, asset-load timing). For runtime evidence,
RUN the app: `cargo run -p grimdark_turfwar --features dynamic_linking`, or a
headless Bevy integration test that drives systems and asserts on world state.
Richer in-engine automation is **TBD (Bevy harness)**.

The ONE definition of green is the full suite, run from the repo root — green
is ALL three passing:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings
cargo test --workspace --features grimdark_turfwar/dynamic_linking
```

(`cargo dclippy` / `cargo dtest` / `cargo drun` in `.cargo/config.toml` are the shorthand.)

The workspace denies clippy all/pedantic/correctness plus
unwrap/expect/panic/todo/unimplemented and missing_docs, so being fmt-clean and
lint-clean ARE part of "done". The repo has zero tests today, so
`cargo test` passes trivially — that is NOT red, but every behavioral ticket
MUST add tests on the real code path.

### Descoping is illegal without the user

You may NOT narrow a clause because the specified design is harder or slower —
that is the lineage's #1 failure mode (`.claude/rules/design-fidelity.md`).
Catching yourself writing "for now", "simplified version", "basic
implementation", "MVP of this" against an in-contract clause means STOP and
ask. The only legal shrink: present the tradeoff → user approves → the Linear
TICKET is updated FIRST → then code. Partial delivery is not delivery; a
ticket lands only when EVERY clause is implemented.

## 8. Finish

Run **/gate**. Its `design-gate` sub-agent audits the diff against your
clause-numbered contract and `docs/` — a change that passes the green suite
but deviates from spec FAILS the gate. Only a passing /gate makes the ticket
eligible for **/land** and for In Review / Done on the board. The
`pre-commit-gate.sh` hook blocks any commit without a matching gate-pass, on a
develop/main branch, or on a red suite. Never claim completion without it.

## Worked example (no argument)

1. PM returns the next ticket: **GTW-207** "battle-sim: resolve a hit along an
   arbitrary attack vector" (no `needs splitting` label → workable).
2. `git status --porcelain` empty, on `develop` → proceed.
3. `git flow feature start gtw-207-attack-vector-resolution` →
   `feature/gtw-207-attack-vector-resolution`.
4. PM moves GTW-207 → In Progress.
5. Contract: **C1** resolve hits along an arbitrary `Vec2`/`IVec2` vector (NOT
   axis-aligned); **C2** deterministic under injected seeded RNG; **C3** per
   `docs/combat/resolution.md`; **C4** no panics on a zero-length vector.
6. GTW-207 depends on GTW-188 (Done) — audit its code matches its claim before
   building on it; `/file-bug` if it shipped narrowed.
7. Implement in `gdtf_battle_sim` with `#[cfg(test)] mod tests` driving the
   real resolution path under a seeded RNG; assert vector math, not a copy.
8. Run the green suite; on green, `/gate`, then `/land`.
