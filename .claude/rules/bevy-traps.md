---
paths:
  - "**/*"
---

# Bevy traps — engine/Rust/wgpu/cargo gotchas that have burned real sessions

Why this rule exists: every item below cost a real debugging session. Check
this list BEFORE chasing "impossible" behavior or filing a bug. This list
starts nearly empty by design — ADD an entry each time a real trap costs a
session, so the next session pays the toll once.

## Bevy 0.18

1. There is NO built-in state-scoped RESOURCE. Entity state-scoping is
   `DespawnOnExit<S>` / `DespawnOnEnter<S>` (renamed in 0.18 — no "State"
   suffix). RESOURCES must be inserted in an `OnEnter(state)` system and
   removed in `OnExit(state)`; any system that touches such a resource must
   guard with `.run_if(resource_exists::<R>)` or take `Option<Res<R>>`, or it
   panics when the resource is absent.
2. Bevy renames/moves APIs across 0.x releases. Pin behavior to the
   `Cargo.lock` Bevy version (currently 0.18.1) and check the migration guide
   for the exact range — do not trust pre-0.18 snippets from memory. When in
   doubt, ask the `bevy-expert` agent.
3. System ordering is AMBIGUOUS without explicit `.before()` / `.after()` /
   `.chain()` or named system sets. Two systems that read+write the same data
   with no ordering will run in a nondeterministic order and produce flaky,
   one-frame-off bugs — order them explicitly.
4. Buffered events are now **messages** in 0.18: the buffered `EventWriter` /
   `EventReader` / `Event` became `MessageWriter` / `MessageReader` / `Message`.
   `AppExit` is a `Message` — to quit, take `MessageWriter<AppExit>` and
   `.write(AppExit::Success)`; `EventWriter<AppExit>` does NOT resolve (E0425).
   The targeted/observer `Event` API is a separate concept now — confirm the
   split via the 0.18 migration guide / the `bevy-expert` agent. [burned GTW-2]
5. A `#[derive(SubStates)]` type registers with `app.add_sub_state::<S>()`, NOT
   `app.init_state::<S>()`. `init_state` **compiles** on a SubStates type (the
   derive also impls `States`/`FreelyMutableState`) but registers it as an
   independent TOP-LEVEL state that activates from startup, ignoring its
   `#[source(...)]` parent — so every sub-state runs un-nested from frame 0 and
   the machine is scrambled (no compile error, no panic). Symptom: child-state
   `OnEnter` logs fire before the parent is ever entered. Also ORDER matters:
   `add_sub_state::<Child>()` must run AFTER the parent state is registered, so a
   parent scene-plugin must `add_*state` for its own state BEFORE `add_plugins`
   pulls in child plugins that register their sub-states.
6. `bevy_ui::Interaction` (None/Hovered/Pressed) is driven by `bevy_ui`'s
   built-in `ui_focus_system` (registered by `bevy_ui::UiPlugin`, in
   `DefaultPlugins`, in `PreUpdate` / `UiSystems::Focus`, `.after(InputSystems)`),
   which reads `ButtonInput<MouseButton>` + the cursor directly and writes
   `Interaction` for every node that has it. It is **NOT** written by
   `UiPickingPlugin` — that is a separate `#[cfg(feature = "bevy_picking")]`
   pipeline that drives `bevy_picking::PickingInteraction` + `Pointer<E>`
   observers, an independent concept. So under `DefaultPlugins`, real mouse
   hover/click ALREADY sets `Interaction::Hovered`/`Pressed` on every `Button`
   (which `#[require]`s `Interaction`): you need NO `UiPickingPlugin`, NO
   `Pickable`, NO `IsDefaultUiCamera` for `Interaction` to work — only a
   `Camera2d`/`Camera` rendering to a window (a windowed `RenderTarget`), which
   the GTW-120 UI camera already is. Consequence: click-to-activate is end-to-end
   for free (`ui_focus_system` sets `Pressed` → GTW-122's `mouse_button_actions`
   reads `Changed<Interaction>==Pressed`); a `Changed<Interaction>` system in
   `Update` sees the same frame's values since `ui_focus_system` ran in the
   earlier `PreUpdate`. Hover→focus, by contrast, has NO built-in bridge — that
   is what GTW-141's `sync_hover_to_focus` adds. [verified GTW-141]

## wgpu / cargo

(none yet — add the first real one here)

## Rust / Bevy style

- Stay clippy-clean under the deny-by-default workspace lints
  (all/pedantic/correctness). No `unwrap`/`expect`/`panic!`/`todo!`/
  `unimplemented!` in shipped code — never propagate `Result`/`Option` just handle
  it. Document every `pub` item (`missing_docs` is denied).

Invite growth: when a Bevy/Rust/wgpu/cargo trap burns a session, append it
here with a one-line "what it looked like vs. what was actually wrong".
