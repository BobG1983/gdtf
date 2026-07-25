# Pointer, keyboard and gamepad picking — spike finding (GTW-811)

Status: **spike finding**, not an implementation. Nothing in the shipped game changed for
this ticket. The only code added is a headless probe suite
(`crates/gdtf_battle_input/tests/picking/ui_picking_coexistence.rs` and
`crates/gdtf_battle_input/tests/picking/ui_widget_pointer_activation.rs`) whose sole job is
to make every claim below re-runnable.

Verified against Bevy **0.19.0** source (crates.io registry copies of `bevy`, `bevy_internal`,
`bevy_ui`, `bevy_picking`, `bevy_input_focus`, `bevy_ui_widgets`) and against the live tree in
this repo on 2026-07-24.

Audience: GTW-840 (the comparison rubric and navigation checklist) and GTW-812 (the
production picking rollout). GTW-840 needs the per-item achievability verdicts in
[§6](#6-the-six-gtw-840-navigation-items); GTW-812 needs the wiring spec in
[§4](#4-recommended-path-and-exact-wiring).

---

## 1. The headline: the premise of the ticket is false

GTW-811 asks whether **enabling** `UiPickingPlugin` would regress the battle
`cursor_over_ui` arbitration. It cannot be enabled, because **it is already enabled, in
every build of this workspace, and has been for as long as the `ui` feature has been on.**

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

`cargo tree -p gdtf_app -e features` confirms the resolved graph contains
`bevy feature "picking"` including `bevy feature "ui_picking"`, and
`bevy_input_focus` features `gamepad`, `keyboard`, `mouse`, `bevy_picking`.

Runtime confirmation:
`ui_picking_coexistence::ui_picking_backend_is_already_installed` asserts
`app.is_plugin_added::<UiPickingPlugin>()` on the standard `DefaultPlugins` headless
harness. It passes.

**Three shipped doc comments are therefore wrong and should be corrected by GTW-812:**

- `crates/gdtf_app/src/states/running/options/systems/actions.rs` — "The project does not
  enable Bevy's `ui_picking` backend (which would let the widgets read pointer events
  themselves), so this bridge is the honest adapter cost…"
- `crates/gdtf_app/src/states/running/options/systems/settings_input.rs` — "no `ui_picking`
  backend is involved". The backend *is* involved: the sound toggle is a real
  `bevy_ui_widgets::Checkbox` with no project-side pointer bridge, so its mouse activation
  today runs through `checkbox_on_pointer_click` and the live backend.
- `crates/gdtf_app/tests/options_scene.rs` carries the same premise in its module doc.

## 2. The two pointer pipelines, mapped (contract clause C4)

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
`Interaction`, which is exactly why GTW-380 wrote it this way.

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

## 3. Answering the engineering question (contract clause C5)

> Determine, against Bevy 0.19 source, whether `UiPickingPlugin` can be enabled such that UI
> clicks are consumed (blocking the act-bus) via picking layers / `cursor_over_ui`, without
> double-firing shots.

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
  GTW-812 must not do that.
- **"Picking layers"** — the phrase in the ticket — is not a Bevy 0.19 facility by that
  name. What exists is `Pickable { should_block_lower, is_hoverable }`
  (`bevy_picking-0.19.0/src/lib.rs`) for depth-blocking *within* the picking pipeline, and
  `UiPickingSettings { require_markers }` (default `false`) for restricting the UI backend to
  `UiPickingCamera` / `Pickable`-marked entities. Neither of those arbitrates between picking
  and a non-picking consumer such as the act bus. There is no built-in cross-pipeline
  "consumed" flag; the arbitration has to be a rule the project states. §5 states it.

### Evidence (contract clause C10)

`crates/gdtf_battle_input/tests/picking/ui_picking_coexistence.rs`, run under the real
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

The pre-existing GTW-380 suite (`crates/gdtf_battle_input/tests/picking/viewport.rs`) still
passes unchanged, so the gate is not regressed under either harness.

## 4. Recommended path and exact wiring

**Recommendation: (i) — `UiPickingPlugin` global (it already is), with the arbitration rule of
§5 made explicit.** Option (ii) "keep the GTW-637-style bridge" is rejected: it was built on a
false premise and costs a translation layer the engine already provides. Option (iii) "scoped
hybrid" is rejected: scoping requires turning the backend *off* somewhere, and it cannot be
turned off without dropping the whole `ui` feature.

Supporting evidence for retiring the bridge —
`crates/gdtf_battle_input/tests/picking/ui_widget_pointer_activation.rs`,
`a_first_party_button_activates_from_a_real_pointer_click`: a bare
`bevy_ui_widgets::Button` with **no project code at all** produces an `Activate` from a real
pointer press/release, via Bevy's own `button_on_pointer_click`. The GTW-637 bridge's mouse
half is redundant today.

### Wiring spec for GTW-812 (contract clause C9)

Everything below is additive to what already exists; nothing needs a manifest change.

1. **Plugins.** No plugin add and no plugin removal. `UiPlugin` (in `DefaultPlugins`) already
   installs `UiPickingPlugin`; `DefaultPlugins` already installs `DefaultPickingPlugins`. Do
   **not** add `UiPickingPlugin` yourself — a second add panics.
2. **Widget activation on meta screens.** The Continue button is *not* a first-party widget
   today: `crates/gdtf_app/src/states/running/options/systems/spawn.rs` builds it with
   `gdtf_ui::spawn_button`, a plain `Node` carrying `Interaction`, so `ButtonPlugin`'s
   pointer observers do not apply to it and the mouse half of `bridge_continue_activation`
   is genuinely load-bearing **as long as that stays true**. The change is therefore a pair,
   in this order: (a) spawn Continue as a `bevy_ui_widgets::Button` (keeping the `gdtf_ui`
   theming), then (b) delete the
   `Query<(Entity, &Interaction), ContinuePressedFilter>` loop from
   `crates/gdtf_app/src/states/running/options/systems/actions.rs`. Doing (b) without (a)
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
   World acts have exactly one producer: `left_click_act` reading `InspectTarget`. Breaking
   this is the only way to create the double-fire the ticket fears, so GTW-812 should carry a
   test asserting a HUD-overlapping click produces no act (its own acceptance 2 already says
   so) and a review note pinning this prohibition.
5. **Optional, only if a HUD widget ever needs the backend to ignore it.** Insert
   `Pickable { should_block_lower: false, is_hoverable: false }` on that node. Do not reach
   for `UiPickingSettings { require_markers: true }` — that is global and would silently
   disable UI picking for every unmarked node in the game.
6. **Gamepad as a pointer (new work, not required by GTW-812).** Register the existing
   `GamepadCursor` as a `bevy_picking` pointer by spawning a `PointerId::Custom(uuid)` entity
   and writing its `PointerLocation` from `move_gamepad_cursor`. Today `GamepadCursor` is
   invisible to `bevy_picking`, so the gamepad cursor can pick board cells but cannot hover or
   click a UI widget.

## 5. The minimum arbitration rule

Stated so GTW-840 can write a checklist item against it:

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

An equivalent formulation, if GTW-812 later prefers to ask the picking pipeline instead of
re-hit-testing: replace the `cursor_over_ui` body with a `Res<HoverMap>` lookup —
`hover_map.get(&PointerId::Mouse).is_some_and(|hits| !hits.is_empty())`. That is
strictly *less* general today, because the gamepad software cursor is not a registered
picking pointer (§4 item 6), so it would silently stop gating gamepad input. Recorded as an
option, **not** recommended until the gamepad pointer is registered.

## 6. The six GTW-840 navigation items

GTW-811's own scope does not name these; they are added here because GTW-840's ruling makes
this finding the input that lets the nav checklist state an achievable bar. Verdicts are for
`bevy_ui` under the recommended path, with `bevy_egui` alongside for comparison.

| Item | `bevy_ui` today | `bevy_ui` achievable | `bevy_egui` | Headless-assertable? |
| --- | --- | --- | --- | --- |
| Tab order | **Absent on meta screens.** `gdtf_ui`'s `FocusNavPlugin` binds Arrow/W/S and D-pad up/down only (`crates/gdtf_ui/src/focus_nav.rs`), and Tab in battle means "cycle ganger" (`crates/gdtf_battle_input/src/act_bus/keyboard.rs`). | **Yes, small.** `bevy_input_focus::tab_navigation::TabNavigationPlugin` + `TabGroup` / `TabIndex` components. Not in `DefaultPlugins` — must be added. Its `handle_tab_navigation` observer is registered on the primary window at `Startup`, so it needs a window. | **Free.** egui handles Tab / Shift-Tab natively (`egui-0.35.0/src/memory/mod.rs`, `Key::Tab` → `FocusDirection::Next` / `Previous`). | **Yes** — assert `InputFocus` moves. Bevy-side needs a synthesized `KeyboardInput` dispatched to the focused entity; egui-side needs an `egui::Event::Key` written into `EguiInput` during `Update`. |
| Visible focus indicator | **Battle only.** `paint_focus_outline` draws an amber `Outline` on the focused panel button (`crates/gdtf_app/src/states/running/game/battlescape/focus_nav/outline.rs`). **No focus indicator exists on the menu or Options screens** — grep finds no `Outline` under those modules. | **Yes, small** — the same system generalized, driven by `InputFocus` (and optionally `InputFocusVisible`, which Bevy's tab nav sets). | egui paints a focus stroke itself from the visuals. | **Partly.** The *component* is assertable headlessly (`Outline` present on exactly the focused entity). That it is *visible on screen* is a pinned-screen-coordinate screenshot claim. Note GTW-782's ring was found uncapturable over `net_qa`'s offscreen capture path. |
| Enter activates | **Yes.** `bridge_keyboard_navigation` raises `FocusActivated` on `Enter`; on Options the GTW-637 bridge turns it into `Activate`. Bevy's own `button_on_key_event` also fires on Enter/Space for a `bevy_ui_widgets::Button` that holds `InputFocus`, with no project code. | Already there. | **Free.** `egui-0.35.0/src/context.rs` treats Space/Enter on a focused clickable as a primary click. | **Yes** both sides. Bevy-side caveat: `FocusedInput` has a private `window` field, so a test cannot construct one — drive `ButtonInput<KeyCode>` plus `InputDispatchPlugin`, or trigger `Activate` directly. |
| Escape cancels | **Battle only, and partial.** `gdtf_ui` registers a `FocusCancelled` message but binds **no key to it**; the battlescape wires Escape context-gated (`crates/gdtf_battle_input/src/act_bus/focus_bridge.rs`). Nothing on the menu/Options screens. | **Yes, small** — bind Escape to `FocusCancelled` per screen and give each screen a cancel consumer. | **Free-ish.** egui clears focus on Escape (`memory/mod.rs`); "close this screen" is still app logic. | **Yes** — assert the state transition / focus clear. |
| Gamepad D-pad | **Yes.** `bridge_gamepad_navigation` maps D-pad up/down to `NavigateRequest` (`crates/gdtf_ui/src/focus_nav.rs`). Left/right are unbound (`NavDirection::WEST` / `EAST` exist but no device input raises them). | Binding left/right is trivial. | **Absent — nothing to build on.** There are **zero** occurrences of "gamepad" in `egui` 0.35.0 or `bevy_egui` 0.41.0 source. Any egui gamepad support must be synthesized by the app: read Bevy `Gamepad` state and push `egui::Event::Key { key: Key::Tab / ArrowDown, .. }` into `EguiInput` during `Update`. That is known to work (GTW-819 drove real egui input this way) but it is app-authored, per-screen, and untested here. | **Yes** on the Bevy side — spawn a `Gamepad` component and assert `InputFocus` moves. **Unknown, likely yes** on the egui side by the `EguiInput` route; this spike did not build it. |
| Gamepad South | **Yes.** `bridge_gamepad_navigation` raises `FocusActivated` on `GamepadButton::South`. Bevy's own widgets do **not** cover this: `button_on_key_event` observes `FocusedInput<KeyboardInput>` only, so the project bridge is load-bearing — which is why §4 item 2 keeps the `FocusActivated` half. `InputDispatchPlugin` does dispatch `FocusedInput<GamepadButtonChangedEvent>` (the `gamepad` feature is on), so a project observer could consume it directly instead. | Already there. | Same as D-pad: **absent**, must be synthesized. | **Yes** on the Bevy side. **Unknown** on the egui side. |

Two cross-cutting notes for the rubric:

- **Structural bias.** Four of the six items are free in egui and per-screen work in
  `bevy_ui`; both gamepad items are free-ish in `bevy_ui` (the framework exists and the
  bridge is written) and absent in egui. A rubric that weights all six equally therefore
  favours egui 4–2 on effort; one that weights *gamepad* heavily inverts it. GTW-840's
  "which stack does this metric structurally favour" clause should record exactly that.
- **In-engine click evidence.** `net_qa` has no mouse-click intent on the wire (only
  `PressKey` / `Hover` / `SetFocus`; `Inject` is battle-only), so a demo build cannot capture
  a real click on a non-menu button over the QA channel. If the checklist wants click
  evidence, it must be either a headless test or a new `net_qa` intent.

## 7. What this spike did **not** determine

Stated explicitly so later tickets do not read inference as fact.

- **In-engine confirmation.** Every claim here is from source reading plus headless probes.
  Nothing was verified by running the real windowed game and clicking. The probes use a
  hand-placed `PointerLocation` / synthesized `PointerInput` because there is no winit event
  loop headlessly; the winit → `PointerInput` translation itself is Bevy's code and is
  untested by this spike. **How to close it:** `cargo drun`, click a HUD panel over a board
  cell, confirm no move/fire; click a board cell, confirm exactly one act.
- **Whether the shipped Options screen mouse-clicks correctly in a real window.** The
  entity shapes were checked and rule out a double-`Activate` today (Continue is a plain
  `Interaction` node driven only by the bridge; the sound toggle is a first-party
  `Checkbox` driven only by the native path), but neither was clicked in the running game.
  **How to close it:** `cargo drun`, open Options, click Continue (expect one return to the
  menu) and click the sound toggle (expect the readout to flip exactly once, not twice).
- **egui gamepad by the `EguiInput` route.** Argued from GTW-819's proven input-injection
  recipe plus the absence of any gamepad code in egui; not built or measured.
- **Whether `TabNavigationPlugin` composes cleanly with the existing
  `DirectionalNavigationMap` topology** the menu, Options and battlescape all write into.
  Both write `InputFocus`; whether their orderings conflict was not tested.
- **Performance.** The picking pipeline's per-frame cost was not measured. It has always been
  running, so it is already priced into current behaviour.

## 8. Contract check

| Clause | Where met |
| --- | --- |
| C1 spike, framework-independent | This document; §6 covers both stacks. |
| C2 no shipped behavior change | No `crates/` or `bins/` source file changed. Additions are two test files plus this document and its index entry. |
| C3 answer before implementing | §1–§3; the GTW-637 premise is refuted in §1 and §4. |
| C4 map both pipelines | §2a, §2b. |
| C5 the engineering question, against 0.19 source | §3, with registry file citations in §1. |
| C6 one recommended path | §4 — path (i), with (ii) and (iii) rejected and why. |
| C7 prototype ceiling | Only tests were written; no production wiring. |
| C8 written finding citing both sources | This document — Bevy 0.19 sources in §1/§3, `cursor_over_ui` in §2a. |
| C9 exact wiring and ordering | §4 "Wiring spec for GTW-812", six numbered items. |
| C10 evidence of no `cursor_over_ui` regression | §3 "Evidence"; the two coexistence tests plus the unchanged GTW-380 suite. |
| C11 deps | None taken; GTW-840 input is §6, GTW-812 input is §4. |
