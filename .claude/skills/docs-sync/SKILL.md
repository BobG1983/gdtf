---
name: docs-sync
description: >-
  Re-align docs/ with the code after sim/behavior changes land. Walks the
  affected design docs claim by claim, verifies every implementation claim
  against the actual Rust source, fixes drift in place, and routes the diff
  through a read-only design-gate review before landing. Use after a feature or
  refactor lands, when docs/ smells stale, or when the user says "sync the docs",
  "docs drifted", or "update the docs to match the code".
argument-hint: "[system or docs/ path, e.g. combat resolution]"
---

# /docs-sync — re-align docs/ with the code

Codebases accumulate docs that go stale while agents claim them current, and
tickets get marked Done for work that never landed. So: **never trust memory,
commit messages, or ticket states — only the Rust code on disk.** Evidence rules
live in `.claude/rules/verification.md`.

## Authority split (binding)

- **Code is the authority for what EXISTS.** Every implementation claim in a doc —
  "X does Y", crate/module paths, type and system names, signatures, formulas,
  magic numbers, schedule/pipeline order — must be verified against the actual
  source before it is allowed to stand.
- **docs/ remain the authority for design INTENT.** If the code is *narrower* than
  the documented design, that is an implementation gap, not a doc bug: leave the
  design text intact, add a status note, and file a gap ticket in Linear (project
  **GDTF**). Do NOT quietly rewrite the design down to match the code — that is the
  exact failure `.claude/rules/design-fidelity.md` exists to prevent.

## Steps

1. **Scope.** Use `$ARGUMENTS` if given. Otherwise identify what changed: the
   just-landed feature diff, `git log --oneline` on `develop` since the last docs
   sync, and recently-Done Linear tickets. Resolve the Linear team that owns
   project **GDTF** via the Linear MCP (do not hardcode a team name); tickets are
   `GTW-N`. Name the changed systems explicitly before touching any docs.
2. **Ticket + branch.** Create or claim a Linear ticket (`GTW-N`) in project GDTF,
   then branch off `develop`: `git flow feature start gtw-N-docs-sync-<slug>` (see
   `.claude/rules/git-workflow.md`). Never edit docs directly on `develop` or `main`.
3. **Map the docs.** Grep `docs/` for the changed systems' names, type names, and
   crate/module paths to find every doc that describes them (start from
   `docs/index.md`). Combat work usually touches `docs/combat/resolution.md`,
   `docs/combat/battle-space.md`, `docs/combat/stats.md`, and `docs/architecture.md`;
   pillar/intent claims live under `docs/pillars/`. List the affected docs before
   editing any of them.
4. **Verify claim by claim.** Walk each affected doc and check every implementation
   claim against the source (`Read` the actual file; note file:line for yourself).
   - The authoritative combat model lives in `crates/gdtf_battle_sim` (render-free);
     its presenter/view mirror lives in `crates/gdtf_battle_presenter`; app shell,
     `AppState`, and scene plugins live in `crates/gdtf_app`. Re-ground any claim to
     real Bevy concepts: ECS systems/components/resources, `Query`/`Commands`,
     schedules, `AppState` transitions (`OnEnter`/`OnExit`), `glam` `IVec2`/`Vec3`.
   - Verdict each claim: **TRUE / DRIFTED / UNBUILT / RETIRED**. A Linear ticket
     saying Done is not evidence; a memory entry is not evidence; only the code is.
   - Verdict semantics:
     - **TRUE** — code matches the claim verbatim (path, name, formula, order). Leave it.
     - **DRIFTED** — code exists but disagrees (renamed type, changed constant,
       reordered schedule). Fix the doc text to the code's reality (step 5).
     - **UNBUILT** — design described, no code backs it yet. Mark "not yet built";
       do not delete the design.
     - **RETIRED** — the approach was removed/replaced. Tombstone it (step 5).
   - Some claims are runtime/visual (presenter, scene transitions, screenshots).
     There is no Godot MCP: verify by running the app — `cargo run -p
     grimdark_turfwar` — or a headless Bevy integration test that drives the schedule
     and asserts state. Richer in-engine automation is **TBD (Bevy harness):**.
5. **Fix in place, keep each doc's voice.** Edit only the false claims; match the
   doc's existing tone, structure, and heading style. Don't rewrite healthy prose,
   don't append changelog noise.
   - Systems designed but not implemented: mark **"not yet built"** explicitly.
   - Retired systems/approaches: mark **RETIRED** with a one-line tombstone of why.
   - Code narrower than design: status note + gap ticket, per the authority split
     above. Never narrow the design text down to match the narrower code.
   - If a Bevy specific cannot be verified from the source or the grimdark design
     intent, mark it exactly **TBD (Bevy):** rather than fabricating it.
6. **Design-gate review (workflow orchestration).** Hand the doc diff (`git diff`)
   plus the list of code files you verified against to the **design-gate** agent
   (`.claude/agents/design-gate.md`) — read-only. Sub-agents cannot spawn
   sub-agents, so the orchestrating workflow / main session invokes design-gate per
   need and relays its verdict. It must independently re-verify every changed claim
   against the code. Fix anything it rejects and re-run it until it passes clean.
7. **Gate, then land — /land owns the commit.** Run **/gate**. The one definition of
   green (run from repo root, all must pass):
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings`
   - `cargo test --workspace --features grimdark_turfwar/dynamic_linking`

   On PASS, run **/land** immediately: /land stages the edited docs **explicitly by
   name** (never `-A` or `.`), commits as `Docs: <summary> (GTW-N)` with a wrapped
   body, finishes the branch off `develop`, and moves `GTW-N` to Done. **Never commit
   yourself between /gate and /land** — a commit moves HEAD and invalidates the gate
   fingerprint, so /land would rightly refuse.

## Notes on Gaps

- Keep docs-sync diffs to `docs/` (plus the occasional `CLAUDE.md` tombstone). Code
  changes do not belong in a `Docs:` commit.
