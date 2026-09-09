# UI pointer picking and act arbitration

How a mouse press is arbitrated between the UI and the battle world: which pointer
pipelines run, which one owns a press, and the wiring that keeps exactly one act producer.

Standing finding, first written when picking was investigated. Verified against Bevy **0.19.0** source
(crates.io registry copies of `bevy`, `bevy_internal`, `bevy_ui`, `bevy_picking`,
`bevy_input_focus`, `bevy_ui_widgets`) and against the live tree on 2026-07-24. Every claim
below is re-runnable through two headless probe suites,
`crates/gdtf_battle_input/tests/battle_input_suite/picking/ui_picking_coexistence.rs` and
`crates/gdtf_battle_input/tests/battle_input_suite/picking/ui_widget_pointer_activation.rs`.

The wiring spec in [§4](#4-recommended-path-and-exact-wiring) is the input to the follow-up
picking / widget rollout work.

---

## 1. The picking backend is already installed

`UiPickingPlugin` cannot be "enabled": **it is already enabled, in every build of this
workspace, and has been for as long as the `ui` feature has been on.**

The chain, read from the registry sources:

1. `Cargo.toml` line 27 pins `bevy = { version = "0.19.0", default-features = false,
   features = ["2d", "ui", "audio"] }`.
2. In `bevy` 0.19.0's own `Cargo.toml`, the `ui` feature expands to
   `["default_app", "default_platform", "ui_api", "ui_bevy_render", "scene", "picking",
   "bevy_ui_widgets"]` — note `picking`.
3. `picking` expands to `["bevy_picking", "mesh_picking", "sprite_picking", "ui_picking"]`.
4. `ui_picking` maps to `bevy_internal/ui_picking`, which is
   `["bevy_picking", "bevy_ui?/bevy_picking", "bevy_input_focus?/bevy_picking"]`.
5. `bevy_ui`'s `UiPlugin::build` (`bevy_ui-0.19.0/src/lib.rs`, the block beginning
   `#[cfg(feature = "bevy_picking")]`) then does
   `app.add_plugins(picking_backend::UiPickingPlugin)` unconditionally, plus
   `widget::viewport_picking` in `First`. The backend's own doc comment says "This is
   included by default in `UiPlugin`".
6. `DefaultPlugins` separately adds `bevy_picking::DefaultPickingPlugins`
   (`bevy_internal-0.19.0/src/default_plugins.rs`, under `#[cfg(feature = "bevy_picking")]`),
   which is `PointerInputPlugin` + `PickingPlugin` + `InteractionPlugin`.

`cargo tree -p gdtf_game -e features` confirms the resolved graph contains
`bevy feature "picking"` including `bevy feature "ui_picking"`, and
`bevy_input_focus` features `gamepad`, `keyboard`, `mouse`, `bevy_picking`.

Runtime confirmation:
`ui_picking_coexistence::ui_picking_backend_is_already_installed` asserts
`app.is_plugin_added::<UiPickingPlugin>()` on the standard `DefaultPlugins` headless
harness. It passes.

**No source comment contradicts this.** Three once said the backend was off:
`crates/gdtf_game/src/states/running/options/systems/actions.rs`,
`crates/gdtf_game/src/states/running/options/systems/settings_input.rs`, and the module doc
of `crates/gdtf_game/tests/game_suite/shell_scenes/options_scene.rs`. None of the three carries that
claim now.

The backend is involved on the Options screen. The sound toggle is a real
`bevy_ui_widgets::Checkbox`
(`crates/gdtf_game/src/states/running/options/systems/spawn/toggle.rs`) with no project-side
pointer bridge, so its mouse activation runs through `checkbox_on_pointer_click` and the live
backend.

## 2. The two pointer pipelines, mapped

### 2a. What the project runs

| Stage | Owner | Where |
| --- | --- | --- |
| OS cursor → UI hover/press state | Bevy `ui_focus_system` (`PreUpdate`, `UiSystems::Focus`) writing `Interaction` | `bevy_ui-0.19.0/src/lib.rs` |
| Gamepad software cursor | `GamepadCursor` + `ActivePointer` (last-moved-wins) | `crates/gdtf_battle_input/src/pointer/gamepad/cursor.rs` |
| Cursor → board cell | `pick_hovered_cell` (`Update`, `InputSystems::Gather`) writing `InspectTarget` | `crates/gdtf_battle_input/src/pointer/picking/hovered.rs` |
| The over-UI gate | `cursor_over_ui` (private fn, called from `resolve_hovered_cell`) | same file, `fn cursor_over_ui` at line 175, called at line 307 |
| Click → act | `left_click_act` → `decide_left_click` (FIRE → SELECT → MOVE → CLEAR) | `crates/gdtf_battle_input/src/pointer/selection/decision/left_click.rs` |
| Act dispatch | `dispatch_act_intents` | `crates/gdtf_battle_input/src/act_bus` |

Ordering is explicit in `crates/gdtf_battle_input/src/plugin/build.rs`: `left_click_act` and
`right_click_turn_to_face` are both `.before(pick_hovered_cell)` and
`.before(dispatch_act_intents)`, so a click acts on the cell resolved *last* update, then the
picker resolves *this* update's cell.

The gate itself is worth quoting because it is the crux of the whole question:

```rust
fn cursor_over_ui(ui_nodes: &Query<UiNodeHit>, cursor: Vec2) -> bool {
    ui_nodes
        .iter()
        .any(|hit| hit.visibility.get() && hit.node.contains_point(*hit.transform, cursor))
}
```

`UiNodeHit` is `(&ComputedNode, &UiGlobalTransform, &InheritedVisibility)`. **It reads no
picking state whatsoever.** It re-does the same `ComputedNode::contains_point` test
`ui_focus_system` does, over every UI node — including bare `Node` panels that carry no
`Interaction`, which is exactly why it was written this way.

### 2b. What `bevy_picking` runs, in parallel, today

`PointerInputPlugin` spawns a `PointerId::Mouse` entity (with required components
`PointerLocation`, `PointerPress`, `PointerInteraction`) and translates winit window events
into `PointerInput` messages. `PickingPlugin` receives them in `PreUpdate`
(`PickingSystems::ProcessInput`); `UiPickingPlugin`'s `ui_picking` system walks the `UiStack`
in `PickingSystems::Backend` and writes `PointerHits`; `InteractionPlugin` turns those into
`HoverMap`, `PickingInteraction` components and the `Pointer<E>` entity-event family
(`Over`, `Out`, `Press`, `Release`, `Click`, `Drag*`, `Scroll`, `Cancel`), all still in
`PreUpdate`, before any `Update` system runs.

**The project consumes none of it.** There are zero occurrences of `bevy_picking`,
`Pointer<`, `PickingInteraction`, `UiPickingPlugin`, `Pickable` or `HoverMap` in any `crates/`
or `bins/` source file. The only project-authored references are the three doc comments in
§1 that (incorrectly) say the backend is off.

So the two pipelines already run side by side and have done for the whole life of the
`ui` feature pin. The risk was never "will enabling it break arbitration"; the shipped game
*is* the experiment, and the answer is that nothing broke, because the two never touch.

## 3. Can UI clicks be consumed without double-firing shots?

The question: can `UiPickingPlugin` be live such that UI clicks are consumed (blocking the
act bus) without a click also firing a shot in the world?

**Yes, and it already is.** Precisely:

- **UI clicks are consumed** by `cursor_over_ui`, which resolves `InspectTarget::hovered()`
  to `None` for a cursor over any visible UI node. `decide_left_click` returns `NoOp` on a
  `None` hovered cell (`left_click.rs`, the `let Some(target) = inspect.hovered() else`
  guard), so no `FireRequested` / `MoveRequested` is ever produced. This is a *pre-emptive*
  gate, not a consume-flag negotiation: the world half never sees the click at all.
- **No double-firing is possible** for a structural reason, not a tuning reason: a second
  fire would require a second producer of an act, and the picking pipeline produces
  `Pointer<E>` entity events that nothing in this workspace observes. Adding a
  `Pointer<Click>` observer that emitted acts is the only way to create the double-fire, and
  the rollout must not do that.
- **"Picking layers"** is not a Bevy 0.19 facility by that name. What exists is
  `Pickable { should_block_lower, is_hoverable }` (`bevy_picking-0.19.0/src/lib.rs`) for
  depth-blocking *within* the picking pipeline, and `UiPickingSettings { require_markers }`
  (default `false`) for restricting the UI backend to `UiPickingCamera` / `Pickable`-marked
  entities. Neither of those arbitrates between picking and a non-picking consumer such as
  the act bus. There is no built-in cross-pipeline "consumed" flag; the arbitration has to be
  a rule the project states. §5 states it.

### Evidence

`crates/gdtf_battle_input/tests/battle_input_suite/picking/ui_picking_coexistence.rs`, run under the real
`DefaultPlugins` headless UI harness with `GdtfBattleInputPlugin` and the real battle
resources:

- `picking_hits_a_panel_while_the_board_click_is_absorbed` — a real laid-out `Node` panel, a
  real `bevy_picking` mouse pointer placed at its centre. Asserts (a) the picking backend
  reports `PickingInteraction::Hovered` on the panel — so the backend is demonstrably *live*,
  not inert; (b) `InspectTarget::hovered()` is `None`; (c) a two-click there emits **no**
  `MoveRequested`.
- `picking_reports_no_hit_off_the_panel_and_the_board_click_lands` — the same app, the same
  pointer, moved off the panel. Asserts picking reports nothing on the panel, the board picker
  *does* resolve a hovered cell, and the two-click commits **exactly one** `MoveRequested`.
  This is what makes the first test non-vacuous and rules out both suppression and
  duplication.

The pre-existing viewport picking suite (`crates/gdtf_battle_input/tests/battle_input_suite/picking/viewport.rs`) still
passes unchanged, so the gate is not regressed under either harness.

## 4. Recommended path and exact wiring

**Recommendation: keep `UiPickingPlugin` global (it already is), with the arbitration rule of
§5 made explicit.** Keeping a hand-rolled pointer bridge instead is rejected: it was built
on a false premise and costs a translation layer the engine already provides. A "scoped
hybrid" is rejected too: scoping requires turning the backend *off* somewhere, and it cannot
be turned off without dropping the whole `ui` feature.

Supporting evidence for retiring the bridge —
`crates/gdtf_battle_input/tests/battle_input_suite/picking/ui_widget_pointer_activation.rs`,
`a_first_party_button_activates_from_a_real_pointer_click`: a bare
`bevy_ui_widgets::Button` with **no project code at all** produces an `Activate` from a real
pointer press/release, via Bevy's own `button_on_pointer_click`. The bridge's mouse
half is redundant today.

### Wiring spec for the rollout

Everything below is additive to what already exists; nothing needs a manifest change.

1. **Plugins.** No plugin add and no plugin removal. `UiPlugin` (in `DefaultPlugins`) already
   installs `UiPickingPlugin`; `DefaultPlugins` already installs `DefaultPickingPlugins`. Do
   **not** add `UiPickingPlugin` yourself — a second add panics.
2. **Widget activation on meta screens.** The Continue button is *not* a first-party widget
   today: `crates/gdtf_game/src/states/running/options/systems/spawn/screen.rs` builds it with
   `gdtf_ui::spawn_button`, a plain `Node` carrying `Interaction`, so `ButtonPlugin`'s
   pointer observers do not apply to it and the mouse half of `bridge_continue_activation`
   is genuinely load-bearing **as long as that stays true**. The change is therefore a pair,
   in this order: (a) spawn Continue as a `bevy_ui_widgets::Button` (keeping the `gdtf_ui`
   theming), then (b) delete the
   `Query<(Entity, &Interaction), ContinuePressedFilter>` loop from
   `crates/gdtf_game/src/states/running/options/systems/actions.rs`. Doing (b) without (a)
   makes the button unclickable; doing (a) without (b) triggers `Activate` twice per click.
   Keep the `MessageReader<FocusActivated>` half either way: that is the **gamepad** path
   (§6), which Bevy does not cover.
   The sound toggle needs no change — it is already a real `bevy_ui_widgets::Checkbox` with
   no project-side pointer bridge (`settings_input.rs`), so its mouse path is already the
   native one and there is nothing to double up.
3. **Battle HUD.** Change nothing in `crates/gdtf_battle_input/src/pointer/picking/hovered.rs`.
   `cursor_over_ui` stays the arbiter. Ordering is already correct and needs no new
   constraint: `ui_picking` and the whole picking pipeline run in `PreUpdate`;
   `pick_hovered_cell` runs in `Update`, so it reads settled `ComputedNode` /
   `UiGlobalTransform` geometry and cannot race the backend.
4. **The one prohibition.** No system or observer in `crates/` may translate a
   `Pointer<Press>` / `Pointer<Click>` / `Pointer<Release>` on a *world* entity into an act.
   A pointer press has exactly one producer: `left_click_act` reading `InspectTarget`. Two
   other callers reach a world act — `gamepad_click_act` on the South button and, in a debug
   build, the `input.click_cell` QA command — and both run the same `decide_left_click` /
   `apply_left_click` pair rather than deciding for themselves, so neither is a second
   decision path and neither reads a `Pointer` event. Breaking the prohibition is the only
   way to create a double-fire, so the rollout should carry a test asserting a
   HUD-overlapping click produces no act, plus a review note pinning it.
5. **Optional, only if a HUD widget ever needs the backend to ignore it.** Insert
   `Pickable { should_block_lower: false, is_hoverable: false }` on that node. Do not reach
   for `UiPickingSettings { require_markers: true }` — that is global and would silently
   disable UI picking for every unmarked node in the game.
6. **Gamepad as a pointer (new work, not required by the picking rollout).** Register the
   existing `GamepadCursor` as a `bevy_picking` pointer by spawning a `PointerId::Custom(uuid)`
   entity and writing its `PointerLocation` from `move_gamepad_cursor`. Today `GamepadCursor`
   is invisible to `bevy_picking`, so the gamepad cursor can pick board cells but cannot hover
   or click a UI widget.

## 5. The minimum arbitration rule

> **One pointer position, one owner per frame.** For a given pointer, the UI owns the press if
> that pointer's position lies inside any visible UI node's laid-out rect; otherwise the world
> owns it. The test is made once per frame, from the same cursor position and the same node
> geometry both halves would use, and the world half is *skipped* rather than
> post-filtered — the world never receives a press the UI owns.

Three properties make it sufficient, and each is checkable:

1. **Single test, single source of geometry.** `cursor_over_ui` uses `ComputedNode` +
   `UiGlobalTransform` + `InheritedVisibility` — the same three inputs `ui_focus_system` and
   `bevy_picking`'s `ui_picking` both hit-test. The three cannot disagree about what is under
   the pointer.
2. **Skip, not undo.** The gate resolves the hovered cell to `None` before any act is
   decided, so there is no act to retract and no ordering hazard between "the UI consumed it"
   and "the world already fired".
3. **Single act producer.** Only `left_click_act` turns a pointer press into a world act.
   Any second producer breaks the rule; §4 item 4 forbids it.

An equivalent formulation, if a later change prefers to ask the picking pipeline instead of
re-hit-testing: replace the `cursor_over_ui` body with a `Res<HoverMap>` lookup —
`hover_map.get(&PointerId::Mouse).is_some_and(|hits| !hits.is_empty())`. That is
strictly *less* general today, because the gamepad software cursor is not a registered
picking pointer (§4 item 6), so it would silently stop gating gamepad input. Recorded as an
option, **not** recommended until the gamepad pointer is registered.

## 6. Keyboard and gamepad activation, today

Recorded because §4 item 2 turns on it: the mouse half of a widget bridge can be retired to
Bevy's own pointer observers, but the keyboard and gamepad halves cannot, and the state below
says which parts exist.

| Item | State today | Achievable | Headless-assertable? |
| --- | --- | --- | --- |
| Tab order | **Absent on meta screens.** `gdtf_ui`'s `FocusNavPlugin` binds Arrow/W/S and D-pad up/down only (`crates/gdtf_ui/src/focus_nav.rs`), and Tab in battle means "cycle ganger" (`crates/gdtf_battle_input/src/act_bus/keyboard.rs`). | **Yes, small.** `bevy_input_focus::tab_navigation::TabNavigationPlugin` + `TabGroup` / `TabIndex` components. Not in `DefaultPlugins` — must be added. Its `handle_tab_navigation` observer is registered on the primary window at `Startup`, so it needs a window. | **Yes** — assert `InputFocus` moves, driving a synthesized `KeyboardInput` dispatched to the focused entity. |
| Visible focus indicator | **Battle only.** `paint_focus_outline` draws an amber `Outline` on the focused panel button (`crates/gdtf_game/src/states/running/game/battlescape/focus_nav/outline.rs`). **No focus indicator exists on the menu or Options screens** — grep finds no `Outline` under those modules. | **Yes, small** — the same system generalized, driven by `InputFocus` (and optionally `InputFocusVisible`, which Bevy's tab nav sets). | **Partly.** The *component* is assertable headlessly (`Outline` present on exactly the focused entity). That it is *visible on screen* is a pinned-screen-coordinate screenshot claim. |
| Enter activates | **Yes.** `bridge_keyboard_navigation` raises `FocusActivated` on `Enter`; on Options the bridge turns it into `Activate`. Bevy's own `button_on_key_event` also fires on Enter/Space for a `bevy_ui_widgets::Button` that holds `InputFocus`, with no project code. | Already there. | **Yes.** Caveat: `FocusedInput` has a private `window` field, so a test cannot construct one — drive `ButtonInput<KeyCode>` plus `InputDispatchPlugin`, or trigger `Activate` directly. |
| Escape cancels | **Battle only, and partial.** `gdtf_ui` registers a `FocusCancelled` message but binds **no key to it**; the battlescape wires Escape context-gated (`crates/gdtf_battle_input/src/act_bus/focus_bridge.rs`). Nothing on the menu/Options screens. | **Yes, small** — bind Escape to `FocusCancelled` per screen and give each screen a cancel consumer. | **Yes** — assert the state transition / focus clear. |
| Gamepad D-pad | **Yes.** `bridge_gamepad_navigation` maps D-pad up/down to `NavigateRequest` (`crates/gdtf_ui/src/focus_nav.rs`). Left/right are unbound (`NavDirection::WEST` / `EAST` exist but no device input raises them). | Binding left/right is trivial. | **Yes** — spawn a `Gamepad` component and assert `InputFocus` moves. |
| Gamepad South | **Yes.** `bridge_gamepad_navigation` raises `FocusActivated` on `GamepadButton::South`. Bevy's own widgets do **not** cover this: `button_on_key_event` observes `FocusedInput<KeyboardInput>` only, so the project bridge is load-bearing — which is why §4 item 2 keeps the `FocusActivated` half. `InputDispatchPlugin` does dispatch `FocusedInput<GamepadButtonChangedEvent>` (the `gamepad` feature is on), so a project observer could consume it directly instead. | Already there. | **Yes.** |

One note on evidence: the MCP channel has no mouse-button press on the wire. `input.hover`
moves the pointer and `input.click_cell` takes the game's own left-click decision on a board
cell, but neither presses a button on a UI widget, so a demo build still cannot capture a real
click on a non-menu button over the channel. Click evidence must be a headless test or a new
command. The keyboard and gamepad halves *are* drivable: `input.focus_step` and
`input.activate` write the same `NavigateRequest` / `FocusActivated` messages the arrow keys
and the D-pad write, and `input.set_focus` puts focus on a widget the screen registered.

## 7. What this finding did **not** determine

Stated explicitly so later work does not read inference as fact.

- **In-engine confirmation.** Every claim here is from source reading plus headless probes.
  Nothing was verified by running the real windowed game and clicking. The probes use a
  hand-placed `PointerLocation` / synthesized `PointerInput` because there is no winit event
  loop headlessly; the winit → `PointerInput` translation itself is Bevy's code and is
  untested here. **How to close it:** `cargo drun`, click a HUD panel over a board
  cell, confirm no move/fire; click a board cell, confirm exactly one act.
- **Whether the shipped Options screen mouse-clicks correctly in a real window.** The
  entity shapes were checked and rule out a double-`Activate` today (Continue is a plain
  `Interaction` node driven only by the bridge; the sound toggle is a first-party
  `Checkbox` driven only by the native path), but neither was clicked in the running game.
  **How to close it:** `cargo drun`, open Options, click Continue (expect one return to the
  menu) and click the sound toggle (expect the readout to flip exactly once, not twice).
- **Whether `TabNavigationPlugin` composes cleanly with the existing
  `DirectionalNavigationMap` topology** the menu, Options and battlescape all write into.
  Both write `InputFocus`; whether their orderings conflict was not tested.
- **Performance.** The picking pipeline's per-frame cost was not measured. It has always been
  running, so it is already priced into current behaviour.
