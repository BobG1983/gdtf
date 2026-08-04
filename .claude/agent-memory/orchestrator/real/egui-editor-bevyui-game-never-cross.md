---
name: egui-editor-bevyui-game-never-cross
description: egui owns the content editor, hand-rolled bevy_ui owns the player-facing game, and neither crosses; game UI needs gamepad focus navigation, mutates in place, and uses relative units from the RON theme.
metadata:
  type: feedback
---

**egui is the content editor. Hand-rolled `bevy_ui` is the game. They do not cross.** No shared
widgets, no migrating one toward the other, no comparison programme — that programme was opened and
then canceled without producing a single measurement.

**Why:** `docs/architecture.md`, "Amendment — 2026-07-25" (line 133),
settles it by user ruling. Clause 1 assigns the stacks, clause 2 says the boundary runs in BOTH
directions (no `gdtf_ui` in the editor, no egui in the player-facing game), clause 3 keeps
`dev_tools`-gated egui inside the game binary, and clause 5 locks `bsn!` authoring for every new
player-facing screen. The amendment also records that the boundary is enforced by review, not by a
conformance test — the user declined the test on purpose.

**How to apply:**

1. **A `dev_tools` egui panel in the game is fine; egui in shipped game UI is not.** The live
   example is `crates/gdtf_app/src/dev/procgen_stepper/`. A release binary never links egui.
2. **Adding egui needs a render app.** `EguiPlugin` reads shader assets, so
   `crates/gdtf_app/src/dev/plugin.rs:12` adds it only `if app.is_plugin_added::<RenderPlugin>()`.
   Adding it to a `MinimalPlugins` test app dies on the first frame.
3. **Gamepad focus navigation and activation are required on the game UI**; gamepad pointer
   emulation is permanently out (the design canon amendment clause 6, line 186). The editor is exempt. The
   game side lives in `crates/gdtf_ui/src/focus_nav.rs`.
4. **UI updates MUTATE in place** — never despawn and respawn a subtree to refresh it. The
   battlescape stat block is the pattern: `stat_block/update.rs` and `writers.rs` rewrite text,
   pips and bar fills through `gdtf_ui`'s `set_progress_bar` / `update_pips`, and the directory
   contains no `despawn` at all.
5. **Relative units, not `Px`.** Layout must reflow:
   `crates/gdtf_ui/src/theming/themed/system.rs:108-114` sizes borders, radii and margins in
   `Val::Vw` / `Val::Vh` from the theme.
6. **The theme is data, hot-reloaded.** `assets/core_tuning/ui_theme.tuning.ron` →
   `crates/gdtf_ui/src/theming/retheme/system.rs`, pinned by
   `crates/gdtf_ui/src/widgets/interaction/test/theme_reload.rs`.
7. **Debug overlays that are noise in normal play stay behind an env flag, default off** —
   `ReachableOverlayEnabled::from_env` in
   `crates/gdtf_battle_presenter/src/overlays/reachable/overlay.rs`, whole impl under
   `#[cfg(debug_assertions)]`.
8. UI is built by discovery — build, play, refine — not from upfront mockups.

The editor is now entirely egui (`crates/gdtf_content_editor/src/egui_shell/`). Its viewport is an
egui image of a rendered preview, and zoom is `cursor_anchored_zoom` driven by egui's
`smooth_scroll_delta` (`egui_shell/prefab/viewport_ui.rs:79-90`) — none of the old `bevy_ui` canvas
mechanics (`UiTransform`, `UiScale`, wheel-as-Message) exist there any more.

Related: [[probe-the-app-dont-grep]], [[settled-calls-dont-repropose]].
