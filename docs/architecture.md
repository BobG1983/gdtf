# Architecture

The two structural rules everything else is built on. Both are checkable against the
tree, which is why they live here rather than in a decision record: if the code stops
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
