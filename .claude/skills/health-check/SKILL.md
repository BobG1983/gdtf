---
name: health-check
description: >-
  Periodic repo health sweep — dead code, real bugs, test gaps — fanned out as
  parallel read-only sub-agents per lens, with a mandatory adversarial refutation
  pass before ANY finding becomes a ticket or a change. Use when the user says
  "health check", "sweep for dead code", "hunt for bugs", "audit test coverage",
  or periodically between features.
argument-hint: "[optional scope, e.g. crates/gdtf_battle_sim]"
---

# /health-check — dead code / bugs / test gaps, adversarially verified

This is a Rust + Bevy 0.18 (ECS) workspace. Its lineage's history includes code
declared "dead" that was actually reached through Bevy indirection (a system
registered via `add_systems`, a type registered for reflection), bugs asserted
without a trace, and happy-path tests graded as coverage. So nothing here is
actioned on one sub-agent's say-so. Evidence rules: `.claude/rules/verification.md`;
design contract: `docs/` (pillars, combat, glossary, litmus-tests, ADRs) +
`.claude/rules/design-fidelity.md`.

This skill is driven by the orchestrating workflow / main session, which spawns
on-demand read-only sub-agents per lens and relays their findings — there is no
persistent team.

## Hard rules (binding)

- **Refute-before-act:** NO ticket, NO deletion, NO fix until the finding has
  survived an independent refutation pass (step 3). **Uncertain = refuted = dropped.**
- Every sweep and refutation sub-agent is **read-only**: instruct it to report only
  and never use Edit/Write or any mutating tool. The workflow acts; they don't.
- Sub-agents cannot spawn sub-agents — the orchestrating workflow / main session
  spawns the sweepers and the refuters itself (Workflow tool / on-demand sub-agents).
- This skill run files tickets; it does not fix. Fixes go through the standard loop
  (step 6), one ticket per branch.

## Steps

1. **Scope.** Use `$ARGUMENTS` if given; otherwise sweep `crates/` and `bins/`
   (skip `target/`). Run all read-only inspection from the repo root
   (`$CLAUDE_PROJECT_DIR`).
2. **Fan out the sweep** — in a single message, spawn three parallel read-only
   sub-agents (Workflow tool / on-demand sub-agents, `general-purpose`), one per lens:
   - **Dead code.** A symbol, system, type, or asset path may be declared dead ONLY
     after the full reference sweep below comes up empty. The sub-agent must report,
     per candidate, each checklist item it ran and what it found:
     1. Compiler/lint signal — build the workspace and read warnings; `dead_code` and
        `unreachable_pub` fire on truly unreferenced items (note: `pub` items exported
        from a crate are NOT flagged, so the compiler alone never proves a `pub` API dead).
     2. Unused dependencies — `cargo machete`, and `cargo +nightly udeps --workspace`
        if a nightly toolchain is present (flags crate-level deps with no use, not fn-level).
     3. Rust identifiers — grep across `crates/` and `bins/` for the item's name: every
        `pub`/`pub(crate)` fn, type, const, trait, and the module path.
     4. **Bevy indirection** (the trap that bit the lineage) — a fn that looks unused may
        be a system registered via `add_systems(OnEnter/OnExit/Update/FixedUpdate, …)`;
        a struct may be an event/resource/component constructed only by Bevy
        (`init_resource`, `add_event`, `EntityCommands`); a type may be live only via
        `register_type::<T>()` / reflection; a plugin may be reached only through
        `add_plugins(…)`. Grep for the type/fn in `add_systems`, `add_plugins`,
        `init_resource`, `init_state`, `add_event`, `register_type`, run-condition
        closures (`run_if`, `in_state`, `resource_exists`), and `OnEnter`/`OnExit` of
        each `AppState` variant.
     5. AppState scene-plugin registration — confirm whether the candidate plugin is
        wired into its scene `plugin.rs` and up through `ScenesPlugin` /
        `GdtfApp::add_plugins`; an unregistered plugin's systems never run but are not
        dead if the scene is designed-dormant (step 4).
     6. Asset / string indirection — grep for asset path strings, label/handle strings,
        and any name passed to Bevy by string rather than by symbol.
     A hit on ANY item = not dead. A skipped item = not dead.
   - **Bugs.** Trace real gameplay scenarios end-to-end through the code (e.g. a hit
     resolved in `gdtf_battle_sim` from the seeded RNG through the resolution pipeline
     to the outcome, then mirrored by `gdtf_battle_presenter`). Report only defects
     pinned to a specific expression: cite file:line, quote the exact expression, and
     give the concrete scenario that reaches it. Example shape of a real finding: "two
     systems both write `Res<…>`/a component with no `.before()`/`.after()`/`.chain()`
     and no shared system set, so ECS runs them in nondeterministic order →
     one-frame-off result on the scenario X" (`.claude/rules/bevy-traps.md` #3). No
     smells, no style notes, no "could be cleaner".
   - **Test gaps.** Enumerate the public surface — `pub`/`pub(crate)` items of each
     crate under `crates/`, prioritizing the authoritative sim (`gdtf_battle_sim`) —
     and grade the tests against it per item: COVERED / WEAK / UNCOVERED. Tests are
     in-crate `#[cfg(test)] mod tests` and integration tests under `crates/<crate>/tests/`;
     happy-path-only = WEAK, not covered. Report the highest-risk uncovered behaviors
     first. The green suite (rule below) is the definition of green, so a gap there is a
     hole in the gate itself. Note: the workspace has few/zero tests today, so a trivially
     green `cargo test` is NOT coverage — grade against the real `pub` surface, not the
     run result.
3. **Refutation pass (mandatory, before any action).** For EVERY finding, spawn an
   independent read-only sub-agent briefed adversarially: "REFUTE this finding. Hunt for
   the evidence that it is wrong — a hidden reference (especially Bevy indirection: an
   `add_systems`/`add_plugins`/`register_type`/`add_event` registration, a `run_if`
   closure, an asset/label string), a guard that prevents the bug, a test that already
   covers it. Confirm only if you cannot." One refuter may take a batch but must verdict
   each finding separately: CONFIRMED or REFUTED, with evidence. Uncertain = REFUTED.
   Discard everything not CONFIRMED — dropping a bad finding is the system working, not a
   failure to report.
4. **Keep-vs-delete (confirmed dead code only).**
   - **Delete** if accidental or superseded (leftover from a retired approach, replaced
     by a newer system).
   - **KEEP** if it is designed-dormant surface — built ahead of a system specified in
     `docs/` (an AppState scene wired but quiet, a sim type reserved for an unbuilt
     resolution step). Check `docs/` before condemning anything; quietly deleting
     specified design is the failure `.claude/rules/design-fidelity.md` forbids. Ticket
     adding a dormancy comment instead, e.g.
     `// Dormant: reserved for <docs/...> <system>; no callers yet (health-check YYYY-MM-DD).`
5. **File tickets.** For each CONFIRMED finding: bugs go through **/file-bug**; dead-code
   removals, dormancy comments, and test-gap work go to the **project-manager** sub-agent
   (Linear project **GDTF**, tickets prefixed **GTW-**; the PM discovers the owning team
   via the Linear MCP — do not hardcode a team name; see
   `.claude/rules/linear-discipline.md`). Put the evidence and the refuter's verdict in
   the ticket body.
6. **Fix via the standard loop — later, not now.** Each ticket is then worked normally:
   **/next-task** → `git flow feature start gtw-N-<slug>` → implement → **/gate** →
   **/land**. Never batch unrelated findings into one tree. `/gate`'s own six-step
   suite (`.claude/rules/verification.md`) is the authoritative green the fix must
   hit — for a quick pre-check while iterating, from the repo root:

   ```bash
   cargo fmt --check
   cargo dclippy
   cargo dtest
   ```

   (This subset is NOT a substitute for `/gate`'s full six-step suite.)

   **Use these aliases (`.cargo/config.toml`), never hand-typed long-form flags** —
   typing them yourself risks silently dropping `dynamic_linking` and falling back to a
   slow static rebuild.
7. **Report.** Per lens: candidates swept / refuted / confirmed / tickets filed (with
   GTW-N ids), plus anything deliberately left alone (designed-dormant surface) and why.
