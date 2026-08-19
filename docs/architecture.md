# Architecture

The four structural rules everything else is built on. All four are checkable against
the tree, which is why they live here rather than in a decision record: if the code stops
matching this page, the page is wrong and gets fixed.

## The model/view split

**The sim owns combat truth. The presenter draws it. The sim never reads the presenter.**

- **Model** — `crates/gdtf_battle_sim`. Render-free, deterministic, unit-testable with
  injected seeded RNG. It knows nothing about pixels, cameras or sprites.
- **View** — `crates/gdtf_battle_presenter`. Mirrors sim state with a one-way dependency.
- **Input** — `crates/gdtf_battle_input`. Sends request messages to the sim; the sim
  answers with result messages.

The dependency chain runs one way: `gdtf_battle_input → gdtf_battle_presenter →
gdtf_battle_sim`. System order is input → sim → presentation.

**What this forbids:** sim logic that reads a `Transform`, a camera, or anything the
presenter owns. Presenter code that decides a combat outcome. A test that proves sim
behaviour by asserting on rendered state.

**Why it is load-bearing:** the sim is the thing that must be deterministic and replayable
from a seed. Every leak of view state into it costs that, and the loss is silent — a
battle that replays differently gives no error.

**Code sites:** the crate boundaries themselves enforce direction, since
`gdtf_battle_sim`'s manifest does not depend on the presenter.

## The presenter plays the log

**Anything the player sees happens when the playback cursor plays the fact, never when the
sim resolves it.**

The sim writes an act log as it resolves. The presenter walks that log at its own pace and
draws what the cursor has reached, so the two clocks are always apart and the sim is always
ahead.

- **The log** — `ActLog` and `ActSeq`, in `gdtf_battle_sim::act_log`.
- **The cursor** — `PlaybackCursor` and `LogPlayhead`, advanced by `advance_playback`.
  `playback_caught_up` and `PlaybackGate` answer whether it has reached the sim.
- **The played fact** — `Played` and `PlayedSignals`. This is what a presenter system
  reads.
- **The mirror** — `DrawnLife`, `DrawnMagazine`, `DrawnPose`, `DrawnPosition` and
  `DrawnVitals`: sim state as played so far, and what the view draws from.

**What this forbids:** a presenter system that reads a raw sim message buffer —
`TerrainPieceDestroyed` and its kin — and draws the result on the spot.
Destruction is in scope: cover, wall and slab, and whatever replaces a destroyed piece.

**Two exceptions the tree still carries**, both dated 2026-08-18 and both on their way out:
`swap_destroyed_slab` reads the raw buffer until GTW-1212 moves it onto the played fact, and
`swap_destroyed_cover` reads the raw buffer until GTW-1175 moves it (clause 15) and then
deletes both (clause 13). Nothing else may join them.

**Why it is load-bearing:** the sim resolves a whole act while playback is still animating
the one before it. A system that reads the sim directly is not slightly early, it is an
arbitrary number of acts early, and it fails silently — nothing errors when a wall breaks
before the shot that broke it. GTW-888 was this bug for hit floating combat text
on the shot path; GTW-889 was consequence text and death ordering; GTW-937 is the
same bug for destruction.

**Code sites:** `crates/gdtf_battle_presenter/src/playback/`.
`crates/gdtf_battle_presenter/src/actors/fx/fct/stacked_reader.rs` is the worked example —
it reads the played fact rather than the sim's buffer. The shot path is pinned the same
way: a raw `ShotFired` pops no hit text. Other raw buffers are still enforced by review.

## The sim answers whether an act may happen

**A gate function in `gdtf_battle_sim` is the one answer to whether an act may happen.
Every consumer calls it. Nobody keeps a second copy of the checks.**

The sim exports two shapes of gate, and which one to call depends on what the caller
needs back:

- **`can_*` returns yes or no.** `can_shove`, `can_melee`, `can_open_door`,
  `can_enter_emplacement`, `can_exit_emplacement`, `can_throw_grenade`, `can_execute`,
  `can_stabilize`, `can_fire`, `can_engage`, `can_move`, `can_reload`, `can_see`,
  `can_set_stance`, `can_set_facing`, `can_spend_tu`.
- **`*_refusal` returns the reason.** `stance_refusal`, `facing_refusal`, `fire_refusal`.
  Each `can_*` over one of them is a thin wrapper, so a caller that has to show or send
  the reason asks the refusal function instead of rebuilding it from a boolean.

**What this forbids:** a panel, a QA command, or a `run_if` that restates a predicate's
terms — the adjacency, the faction, the life state, the magazine, the TU — instead of
calling it. It also forbids answering only part of a predicate: an offer that checks the
target side and skips the actor side has still made up its own gate.

Some acts are offered while they cannot be paid for, greyed rather than hidden. That does
not license a private affordability check. Where the predicate folds TU in, ask it twice:
once with the act's own cost as the pool, which holds the affordability term true so the
rest picks the target, then again with the real pool to decide whether the button is
pressable. Where the predicate is TU-blind — `can_shove`, `can_melee` — ask it for the
target and ask `can_spend_tu` for the pool. Either way both answers come from the sim.

**Why it is load-bearing:** a copy starts out agreeing and stops agreeing on the next
change to the sim, and it fails quietly — the player is offered an act the sim then
refuses, or is denied one it would have allowed. Nothing errors either way.

**Code sites:** the predicates live beside the act they gate, under
`crates/gdtf_battle_sim/src/acts/`, `.../combatants/`, `.../equipment/magazine/` and
`.../perception/los/` (`can_see`). Consumers:
`crates/gdtf_app/src/states/running/game/battlescape/contextual_panel/acts/`,
`crates/gdtf_app/src/dev/net_qa/commands/`, and
`crates/gdtf_battle_input/src/pointer/fire_surface.rs`.

## The UI boundary

**The game UI is `bevy_ui`. The content editor is `egui`. They never cross.**

- Game UI — `bevy_ui` node/widget primitives, plus Bevy's own first-party
  `bevy_ui_widgets` where they fit. Themed from data, loaded from RON.
- Editor UI — `bevy_egui`, in `crates/gdtf_content_editor`.

**No third-party UI ecosystem crates.** Not `iyes_*`, not `bevy-ui-*`, no external widget,
styling or layout framework. Bevy's own first-party widgets are preferred over
hand-rolling; the ban is on the third-party ecosystem, not on Bevy's primitives.

**What this forbids:** egui anywhere in the game's screens, `bevy_ui` anywhere in the
editor, and a shared widget layer spanning both. Gamepad focus navigation is required on
the game UI; pointer emulation is not a substitute.

**Why it is load-bearing:** the two stacks have incompatible layout and input models.
A widget that tries to serve both ends up serving neither, and the failure shows up as
input that works with a mouse and not a controller.

## The app state tree

`AppState` drives the whole app, with nested `SubStates` beneath `Running`. Each layer is
a `#[derive(SubStates)]` registered with `add_sub_state`, never `init_state`, nested under
its `#[source(...)]` parent.

```text
AppState        Init | Load | Intro | Running | Teardown        (Init is #[default])
  RunningState      Menu | Game | Options | Quit                 ← from AppState::Running
    GameState           Setup | HiveScape | BattleScape          ← from RunningState::Game
      BattleScapeState      Generation | AnimateIn | BattleRunning | AnimateOut | AfterMath
        AfterMathState          AnimateIn | DisplayAftermath | AnimateOut
```

The `HiveScape` / `BattleScape` / `AfterMath` vocabulary is defined in
[glossary.md](glossary.md).

**Code site:** `crates/gdtf_app/src/states/`.
