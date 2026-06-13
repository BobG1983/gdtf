---
name: file-bug
description: >-
  File a defect with root-cause evidence, then fix it through the standard
  loop — investigate to file:line FIRST, file the Linear bug (shipped-claim vs
  actual, root cause, fix contract, test plan), THEN branch, write a failing
  regression test, fix, /gate, /land. Use when behavior contradicts a shipped
  ticket or a design doc, or the user reports something broken.
argument-hint: "[summary]"
---

# /file-bug — root cause first, ticket second, fix third

This is a Rust + Bevy 0.18 ECS workspace. The orchestrating workflow / main
session invokes the sub-agents named below per-need and relays their findings
to the user — there is no persistent team.

What counts as a bug worth filing here: any behavior that contradicts a
shipped GTW-N ticket's claim, or that diverges from the design canon in
`docs/` (pillars, combat specs, glossary, litmus-tests, ADRs, architecture).
A **silently narrowed design** — code that does less than the ticket or doc
specified — is a defect even if it compiles and the suite is green
(`.claude/rules/design-fidelity.md`). Crashes, panics, wrong sim results, and
presenter/sim divergence all qualify.

Hard rules (non-negotiable):

- **Never fix-then-file.** The bug ticket exists BEFORE the first line of fix
  code. A post-hoc ticket is a fig leaf, not a record — the lineage's history
  of invisible quiet fixes is why this order is mandatory.
- **Never fold a bugfix into an unrelated branch.** One bug = one ticket = one
  branch. Folding fixes into whatever branch happens to be open is how
  unattributable multi-ticket trees accumulated in the lineage.

## 1. Investigate the root cause FIRST

- Reproduce or pin down the broken behavior. No fixing yet. Reproduce via a
  failing assertion against `gdtf_battle_sim`, or by running the app
  (`cargo run -p grimdark_turfwar --features dynamic_linking`) and observing the live behavior.
- Find the exact mechanism: cite **file:line** evidence for the defect (e.g.
  `crates/gdtf_battle_sim/src/<module>.rs:NN`).
- Find the ticket that shipped the behavior: `git log -S'<token>'` /
  `git blame` the offending lines, match the commit's `(GTW-N)` subject tag to
  its Linear ticket. That ticket's claim is the "shipped claim".
- If the root cause is a **narrowed design** (code does less than the ticket
  or design doc specified — see `.claude/rules/design-fidelity.md`), say so
  explicitly and cite the design source in `docs/` (e.g.
  `docs/combat/resolution.md`, `docs/combat/battle-space.md`,
  `docs/architecture.md`, the relevant ADR under `docs/decisions/`).

## 2. File the Linear bug

Via the **project-manager** sub-agent (Agent tool), create the bug in project
**GDTF** (the PM discovers the owning team via the Linear MCP — do not hardcode
a team name), with these four sections:

- **Shipped claim vs actual** — what GTW-N claimed Done, what the code does.
- **Root cause** — the file:line mechanism from step 1.
- **Fix contract** — clause-numbered (C1, C2, …): exactly what the fix changes
  and what it must NOT change.
- **Test plan** — the regression test(s) that fail before the fix and pass
  after, exercising the real code path (Rust `#[cfg(test)]` / integration
  tests — NOT `res://test`).

## 3. THEN branch

Require a clean tree on `develop` (`git status --porcelain` empty,
`git branch --show-current` is `develop`); refuse otherwise: finish or stash
first. Then `git flow feature start gtw-N-slug` using the NEW bug ticket's
number → `feature/gtw-N-slug` off `develop`. (The kit lands everything through
the feature flow — **/land** finishes feature branches.) Have the
project-manager sub-agent move the bug to **In Progress** per
`.claude/rules/linear-discipline.md`.

## 4. Implement

Write the regression test FIRST and watch it **fail for the root-cause
reason** — a Rust `#[cfg(test)] mod tests` unit test or an integration test
under `crates/<crate>/tests/`, driving the same systems/components/resources
the app runs (sim logic with injected seeded RNG); never a reimplementation of
the logic inside the test. Then fix; then watch the test pass. Tests exercise
the real code path per `.claude/rules/verification.md`.

The ONE definition of green is the full suite, run from the repo root — green
is ALL three passing:

```
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings
cargo test --workspace --features grimdark_turfwar/dynamic_linking
```

(`cargo dclippy` / `cargo dtest` / `cargo drun` in `.cargo/config.toml` are the shorthand.)

The workspace denies clippy all/pedantic/correctness plus
unwrap/expect/panic/todo/unimplemented and missing_docs, so being fmt-clean and
lint-clean ARE part of "done".

Stage files explicitly by name; never `git add -A` / `git add .`. Use a commit
subject `Area: summary (GTW-N)` referencing the bug ticket
(`.claude/rules/git-workflow.md`). View / presenter defects additionally
require in-engine evidence: run the app and observe the corrected behavior, or
a headless Bevy integration test — richer automation is **TBD (Bevy harness)**.

## 5. Gate, then land

Run **/gate** — its `design-gate` sub-agent verifies the fix against the bug's
fix contract first-hand, and the `pre-commit-gate.sh` hook blocks any commit
without a matching gate-pass; on pass, run **/land**. Board states move only as
they become true — never ahead of the code
(`.claude/rules/linear-discipline.md`).

## Worked example

1. The sim resolves a hit only along the X axis even though GTW-207 claimed
   arbitrary-vector resolution. Reproduce with a failing assert in
   `gdtf_battle_sim` for a 45° vector. No fixing yet.
2. Root cause: `crates/gdtf_battle_sim/src/resolution.rs:142` snaps the vector
   to the dominant axis — a narrowed design vs `docs/combat/resolution.md`.
   `git blame` → commit `Combat: resolve hits along attack vector (GTW-207)`
   → the shipped claim is GTW-207.
3. PM files **GTW-231** in project GDTF with the four sections; the Fix
   contract: **C1** resolve along the true `Vec2` vector; **C2** must NOT
   change axis-aligned results that were already correct; **C3** no panic on a
   zero-length vector.
4. Clean tree on `develop` → `git flow feature start gtw-231-vector-snap-fix`;
   PM moves GTW-231 → In Progress.
5. Write the failing regression test first (the 45° case fails for the
   axis-snap reason), fix `resolution.rs`, watch it pass, run the green suite,
   `/gate`, `/land`.
