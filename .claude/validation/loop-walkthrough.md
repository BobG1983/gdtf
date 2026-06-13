# Loop walkthrough — one GTW- ticket, end to end

**What this is:** a DESIGN-TRACE, not a transcript. It walks one *hypothetical*
small ticket through the gdtf workflow loop (`/next-task` → implement → `/gate`
→ `/land`) and names, at each step, the kit file that scripts the behavior. No
real ticket was run; SHAs, clause text, and counts below are illustrative. The
goal is to confirm the four skills + rules + hook compose into a coherent loop
and that the defect lessons inherited from grimdark are each owned by some step.

The orchestration model is **Workflows**, not a standing roster: each step is
run by the orchestrating session, which spawns a sub-agent *per need* (Agent
tool) and relays the result to the user. There is no persistent PM role and no
named-message mesh — see "Workflow model vs. grimdark" at the end.

## The hypothetical ticket

**GTW-231** "battle-sim: clamp knockback to the battle-space bounds" — small,
not `LARGE`-labelled, lives in the render-free `gdtf_battle_sim` crate. Used
purely to exercise the loop.

## Step 1 — `/next-task` (start the ticket the disciplined way)

Scripted by `.claude/skills/next-task/SKILL.md`. In order:

1. **Pick** — orchestrating session invokes the **project-manager** sub-agent
   for the next ticket in priority order, Linear project **GDTF**. The PM
   discovers the owning team via the Linear MCP; no team name is hardcoded
   (`.claude/rules/linear-discipline.md`). A `LARGE` pick is refused and
   returned for decomposition. GTW-231 is small → workable.
2. **Clean start** — `git status --porcelain` empty AND
   `git branch --show-current` == `develop`, else **refuse**. This is the
   anti-monster-tree guard (`.claude/rules/git-workflow.md`).
3. **Branch** — `git flow feature start gtw-231-clamp-knockback` →
   `feature/gtw-231-clamp-knockback` off `develop`.
4. **Move ticket** — PM sub-agent moves GTW-231 → **In Progress** so the board
   mirrors reality (`linear-discipline.md`, rule 2).
5. **Contract** — restate the ticket as clause-numbered C1, C2, … BEFORE any
   code. E.g. **C1** knockback destination is clamped into the battle-space
   bounds per `docs/combat/battle-space.md`; **C2** deterministic under
   injected seeded RNG; **C3** no panic on an out-of-bounds origin. The
   contract is bound jointly to the ticket AND `docs/` canon
   (`.claude/rules/design-fidelity.md`, rule 4) — this is the anti-narrowing
   device.
6. **Audit "Done" deps** — any dependency marked Done is audited against real
   code before building on it (Done is a claim, not proof —
   `design-fidelity.md` rule 3); the **design-gate** sub-agent may do the audit.
7. **Implement** — to the contract; sim logic in `gdtf_battle_sim` with
   `#[cfg(test)] mod tests` driving the **real** code path under seeded RNG
   (`.claude/rules/verification.md` rule 2). Descoping is illegal without the
   user; the stop-signal phrases ("for now", "simplified version", …) are tripwires.

## Step 2 — implement to green

Scripted by `.claude/rules/verification.md` (the definition of done) and
`.claude/rules/bevy-traps.md` (ECS gotchas). The **one definition of green**,
run from the repo root, is all three passing:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings
cargo test --workspace --features grimdark_turfwar/dynamic_linking
```

(`cargo dclippy` / `cargo dtest` / `cargo drun` in `.cargo/config.toml` are the shorthand.)

Because the workspace `Cargo.toml` denies clippy all/pedantic/correctness plus
unwrap/expect/panic/todo/unimplemented and missing_docs, **fmt-clean and
lint-clean ARE part of done**, not extras. gdtf has ~zero tests today, so
`cargo test` passes trivially — that is NOT red, but a behavioral ticket whose
clauses add no test on the real path is a contract VIOLATION at the gate, not a
green light.

## Step 3 — `/gate` (contract check before landing)

Scripted by `.claude/skills/gate/SKILL.md`, which spawns the **design-gate**
sub-agent (`.claude/agents/design-gate.md`). Sequence:

1. Resolve GTW-231 (arg or from branch name); pull the FULL ticket via Linear
   MCP (team discovered, not hardcoded) — never gate from memory.
2. Run the green suite; red → the gate FAILS immediately.
3. Restate the read-only clause-numbered contract (faithful to ticket + `docs/`).
4. **Gather the diff including untracked files**:
   `git status`, `git diff develop...HEAD`, and
   `git ls-files -o --exclude-standard`. (Untracked content is part of the change.)
5. Spawn **design-gate** per need; pass it the ticket id, contract, touched
   `docs/` paths, the diff, and the verbatim instruction to verify every clause
   FIRST-HAND (read code, run `cargo run -p grimdark_turfwar --features dynamic_linking` or a headless
   Bevy integration test) — trusting nothing the implementer claimed. The
   sub-agent reports back; it does not spawn further sub-agents.
6. Relay the per-clause PASS / VIOLATION verdict; a VIOLATION is never
   editorialized into a pass.
7. On violations, repair the **CODE** — editing the contract, weakening
   criteria, or narrowing the design to pass is forbidden
   (`.claude/rules/design-fidelity.md`). Max 2 repair rounds, then STOP.
8. **On full PASS, write the fingerprint** `.claude/.gate-pass` (see below) and
   move GTW-231 → **In Review** via Linear MCP. `/gate` NEVER moves to Done.

## Step 4 — `/land` (finish into develop) — the ONLY step that commits

Scripted by `.claude/skills/land/SKILL.md` + `.claude/rules/git-workflow.md`;
git plumbing may go through the **source-control** sub-agent. Preconditions are
checked and any failure REFUSES loudly: gate passed this session for THIS tree
(fingerprint recomputed and matched), suite green NOW, on the `feature/*`
branch, no out-of-scope files, every clause implemented (no `todo!`/stub), and
ticket scope not shrunk mid-session.

Then: stage **explicit files by name** (never `git add -A/.`), commit in house
style `Area: summary (GTW-231)`, record `OLD=$(git rev-parse origin/develop)`,
`GIT_EDITOR=true git flow feature finish gtw-231-clamp-knockback` (rebase →
merge into develop → delete branch), `git push origin develop`, move GTW-231 →
**Done** via Linear MCP with an evidence note (merge SHA, suite result, pushed
range), report the range, then **delete** `.claude/.gate-pass`.

## The `.gate-pass` fingerprint — the seam between `/gate` and `/land`

`/gate` writes `.claude/.gate-pass` with TICKET / BRANCH / HEAD / FINGERPRINT
lines. The fingerprint is computed by:

```bash
{ git rev-parse HEAD; git status --porcelain; git diff HEAD; \
  git ls-files -o --exclude-standard -z | LC_ALL=C sort -z \
    | xargs -0 -r shasum -a 256; } | shasum -a 256 | cut -d' ' -f1
```

`/land` precondition 1 recomputes that exact command and refuses on any
mismatch. The `.claude/hooks/pre-commit-gate.sh` PreToolUse hook is the
deterministic backstop: it blocks every commit unless `.claude/.gate-pass`
exists, names the current branch, and its recorded HEAD is an
ancestor-or-equal of the current HEAD (so `/land`'s multiple per-concern
commits all pass off one gate), AND the branch is not develop/main, AND the
green suite is currently green.

## Defect lessons carried forward from grimdark (each must stay owned)

These are the failure modes the grimdark loop hit; verify gdtf still defends
each:

- **The fingerprint must not perturb itself.** `.claude/.gate-pass` is
  gitignored (`.claude/.gitignore` lists `.gate-pass`), so writing it does NOT
  change `git status --porcelain` / `git ls-files -o` and therefore cannot
  alter the very hash it records. Both `/gate` step 9 and `/land` precondition 1
  call this out explicitly. ✔ owned.
- **It must cover untracked content.** The fingerprint hashes the CONTENT of
  every untracked file via `git ls-files -o --exclude-standard -z | … | shasum`,
  so an all-untracked tree (common when a ticket adds new `.rs` files) is fully
  covered — not just `git diff HEAD`, which would miss new files entirely. ✔ owned.
- **Some step must own the In-Review transition.** `/gate` step 10 moves
  GTW-N → In Review on full PASS; `/land` step 6 moves it → Done. Neither
  `/next-task` (which only sets In Progress) nor the hook touches In Review, so
  there is exactly one owner and no gap (`linear-discipline.md` rule 2). ✔ owned.
- **Only `/land` commits.** `/next-task` and `/gate` never commit; the
  pre-commit hook blocks commits without a matching gate-pass and on
  develop/main; `git-workflow.md` rule 6 makes `/land` the only path to
  develop. The **source-control** sub-agent also commits only gate-passed work.
  ✔ owned.

## Workflow model vs. grimdark — where this loop differs

- **Sub-agents are spawned per workflow step, not standing roles.** Where the
  grimdark loop phrased coordination as a persistent lead relaying to the user
  and delegating to a named peer, gdtf reads "the orchestrating workflow / main
  session invokes X and relays". project-manager, design-gate, and
  source-control are each spawned on demand by a step and report back — there
  is no persistent roster and no by-name peer-messaging mesh.
- **The "subagents cannot nest" caveat may not bite the same way.** The
  grimdark loop leaned on a single persistent lead fanning out to peers; here
  each skill step is itself the orchestrator and spawns exactly the sub-agent
  it needs.
  Where a deeper chain is genuinely required, the skills constrain it
  explicitly — e.g. `/gate` instructs design-gate that it "does not spawn
  further sub-agents" and reports back to the spawning session. So the loop is
  designed to stay one level deep per step rather than relying on a nesting
  guarantee.
- **Runtime evidence is lighter, by design.** There is no Godot MCP /
  `play_scene` / `simulate_input`. Runtime checks RUN the app
  (`cargo run -p grimdark_turfwar --features dynamic_linking`) or a headless Bevy integration test that
  drives systems and asserts on world state; richer in-engine input automation
  is **TBD (Bevy harness)** until one exists.
