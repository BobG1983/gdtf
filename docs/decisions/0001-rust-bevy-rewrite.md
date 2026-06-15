---
name: "ADR 0001: Reimplement grimdark-turfwar in Rust + Bevy"
description: Reimplement the Godot grimdark-turfwar game in Rust + Bevy 0.18, preserving the design canon and the model/view split.
---

# 0001. Reimplement grimdark-turfwar in Rust + Bevy

## Status

`Accepted` — 2026-06-12.

## Context

The game (a turn-based tactics *situation generator* — Necromunda campaign ×
XCOM) was originally built in Godot with GDScript, in the
`grimdark-turfwar` repo. The **design** is mature and not in question: the
thesis, the core loop, and the eight design pillars
([../pillars/index.md](../pillars/index.md)) are locked, and the combat rules
([../combat/](../combat/index.md)) are fully specified. The Godot
implementation also proved out a clean **model/view split** — a render-free
authoritative combat sim (`src/combat`) mirrored by a presentational
battlescape scene that owns no gameplay facts.

What was in question was the **engine**. The reasons to leave Godot/GDScript:

- The combat sim is the heart of the game and must be deterministic and
  exhaustively unit-testable. Rust's type system, ownership model, and `cargo
  test` make a render-free authoritative sim far easier to keep honest than
  dynamically-typed GDScript on data.
- A strict, lint-enforced, statically-typed codebase (see ADR 0002) catches at
  compile time the class of bugs that GDScript surfaces only at runtime.
- Bevy's ECS is a natural fit for the model/view split already validated in the
  Godot build: systems and resources for the sim, a mirroring presenter for the
  view, with a one-way dependency.

What carries over unchanged is the **design canon** — it is engine-agnostic and
is the contract for the rewrite: `docs/pillars/`, `docs/combat/`,
`docs/glossary.md`, and `docs/litmus-tests.md`. The model/view architecture it
proved is recorded in this ADR's [Decision](#decision) and
[Consequences](#consequences) (the standalone `docs/architecture.md` was
deliberately removed; its content now lives here and in
[`../glossary.md`](../glossary.md)).

## Decision

We will reimplement grimdark-turfwar from scratch in **Rust + Bevy 0.18**
(0.18.1) as a new project, **GDTF** (GrimDark TurF war), in the `gdtf` repo.
This is a clean-room engine rebuild, **not** a port of GDScript:

- The app is a Bevy `App` driven by a top-level `AppState` enum
  (`Init`, `Load`, `Intro`, `Running`, `Teardown`; `Init` is `#[default]`),
  laid out as a cargo workspace (`crates/*`, `bins/*`, resolver 3). `Running`
  fans out into a four-layer Bevy `SubStates` tree — each layer a
  `#[derive(SubStates)]` registered with `add_sub_state` (NOT `init_state`; see
  [`bevy-traps.md`](../../.claude/rules/bevy-traps.md) #5), nested under its
  `#[source(...)]` parent:
  - `RunningState{Menu, Game, Options, Quit}` — sourced from `AppState::Running`
    (the main-menu screen is `RunningState::Menu`).
  - `GameState{Setup, HiveScape, BattleScape}` — sourced from
    `RunningState::Game`.
  - `BattleScapeState{Generation, AnimateIn, BattleRunning, AnimateOut,
    AfterMath}` — sourced from `GameState::BattleScape`.
  - `AfterMathState{AnimateIn, DisplayAftermath, AnimateOut}` — sourced from
    `BattleScapeState::AfterMath`.

  The `HiveScape` / `BattleScape` / `AfterMath` vocabulary is defined in
  [`../glossary.md`](../glossary.md).
- The authoritative, render-free combat sim — the **model** — lives in
  `crates/gdtf_battle_sim`, deterministic and unit-testable with injected
  seeded RNG.
- The **view/presenter** lives in `crates/gdtf_battle_presenter`, mirroring sim
  state with a one-way dependency (presenter reads the sim; the sim never reads
  the presenter).
- The design canon is ported into `gdtf` `docs/` and remains the source of
  truth; the rewrite implements exactly what it specifies.

## Consequences

- The **engine layer is rebuilt** in Bevy ECS: scenes become `AppState`
  plugins, the Godot `src/combat` model becomes the `gdtf_battle_sim` crate,
  and the battlescape scene becomes the `gdtf_battle_presenter` crate. Godot
  artifacts (`.tscn`, `.tres`, TileMap, `project.godot`, GUT, `res://`) have no
  equivalent and do not carry over.
- The **design pillars and combat rules are preserved** verbatim as the
  contract — the rewrite changes *how* the game is built, never *what* it is.
- The model/view discipline is preserved and tightened: the one-way
  presenter→sim dependency is now enforceable by the crate graph.
- Determinism and testability improve: sim logic is render-free Rust, exercised
  by in-crate `#[cfg(test)]` tests and `crates/<crate>/tests/` integration
  tests with seeded RNG — no scene tree required.
- Cost: the engine layer is reimplemented from zero. Tooling, asset pipeline,
  and runtime evidence move to the Bevy/cargo world (run the app via `cargo run
  -p grimdark_turfwar`; richer in-engine automation is **TBD (Bevy harness)**).

### The landed E10 sim↔app wiring (headless)

The model/view boundary the deleted `docs/architecture.md` used to describe is
landed (epic E10) as a **headless, message-driven seam** — no renderer, no
window, no camera, no presenter visuals are part of it (`gdtf_battle_presenter`
remains a stub; the CP437 renderer and the `gdtf_battle_input` crate are a
later, separate epic, not E10 work). The shape:

- **`gdtf_battle_sim::BattleSimPlugin`** is the SIM-owned registration unit:
  adding this one plugin wires the whole render-free combat runtime into a Bevy
  `App`. It bundles `OccupancyMaintenancePlugin` (the occupancy/cover
  maintenance layer) and `SimActsPlugin` (the per-act dispatch), and registers
  the lifecycle messages. It names NO `gdtf_app` type, preserving the one-way
  `gdtf_app -> gdtf_battle_sim` dependency.
- **The message-driven boundary.** The app drives the sim through buffered Bevy
  `Message`s (the `bevy-traps.md` #4 buffered `Message`, NOT the observer
  `Event`):
  - **Lifecycle:** the app sends `SetupBattleRequested` (carrying an owned
    `Situation` + a `BattleSeed`) and `TeardownBattleRequested`; the sim
    replies with `BattleReady` on a successful setup (fail-closed: a bad battle
    emits no `BattleReady`).
  - **Per-act input contract:** the app sends `*Requested` act messages —
    `FireRequested`, `SetStanceRequested`, `SetFacingRequested`,
    `SetAimingRequested`, `StabilizeDownedRequested`, `ExecuteDownedRequested` —
    each a Bevy `Message` consumed by `SimActsPlugin`'s dispatch systems. There
    is deliberately **NO `MoveRequested`** — no movement act exists yet.
- **The simulate band.** The bundled dispatch + maintenance systems run in the
  `SimSystems::Simulate` `SystemSet`, gated `run_if` on the `BattleInProgress`
  witness (a zero-sized marker `Resource` present exactly while a battle is
  live), so the runtime stays inert and panic-free outside a battle.
- **The app's thin glue.** On entering `BattleScapeState::Generation` the app
  sends `SetupBattleRequested` (from its loaded situation), gates the
  generation-complete advance on receiving `BattleReady`, and on
  `OnExit(GameState::BattleScape)` sends `TeardownBattleRequested`.
  `BattleScapeState::BattleRunning` carries a turn-budget completion gate
  (`BattleRunTurnBudget`).

Known follow-up (out of scope for the docs reconcile that recorded this note):
~48 in-source `gdtf_battle_sim` `.rs` doc-comments still reference the deleted
`docs/architecture.md` path; repointing those source comments is tracked
separately (GTW-211).

## Alternatives considered

- **Stay on Godot / GDScript.** Rejected. The existing implementation works and
  the design is proven, but the dynamically-typed GDScript model makes the
  authoritative sim harder to keep deterministic and exhaustively tested, and
  forfeits the compile-time guarantees and strict-lint discipline (ADR 0002)
  that the project wants around its combat truth. The design canon and the
  model/view lessons it produced are kept; the engine is not.
