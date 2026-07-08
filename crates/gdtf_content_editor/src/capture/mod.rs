//! QA / debug-only self-screenshot-then-exit affordance for the editor (GTW-417 AC4; re-pointed
//! onto the MODEL in the GTW-512 egui swap; split by concern in GTW-574).
//!
//! OFF BY DEFAULT: with no `GDTF_EDITOR_SHOT` env var the editor launches normally — interactive, no
//! screenshot, no auto-exit. When `GDTF_EDITOR_SHOT=/abs/out.png` is set, the editor — once it
//! reaches [`Editing`](crate::EditorState::Editing) and the egui layout has settled — captures the
//! primary window to that PNG and exits cleanly. This lets the gate's Screenshot-QA phase capture the
//! egui shell unattended while keeping the SHIPPED editor clean (the env-gated, inert-by-default
//! affordance precedent).
//!
//! ## GTW-513 C2.4: capture mode-force
//!
//! A second, optional env var — `GDTF_EDITOR_MODE` (`terrain` | `theme` | `prefab` | `gang` |
//! `armor` | `injury` | `sprite` | `attachment`, case-insensitive) — FORCES the
//! [`EditorMode`](crate::EditorMode) before the settle / screenshot,
//! so a QA run can capture a SPECIFIC Workbench mode (e.g. the TERRAIN form). It is honored ONLY
//! when the capture affordance itself is enabled (`GDTF_EDITOR_SHOT` set); unset or an unrecognized
//! value keeps the editor's default mode (the pre-C2.4 behavior). Example:
//! `GDTF_EDITOR_SHOT=/abs/terrain.png GDTF_EDITOR_MODE=terrain cargo run -p gdtf_content_editor_bin`.
//!
//! ## GTW-574: TERRAIN kind pre-select (the Emplacement QA drive)
//!
//! A further optional env var — `GDTF_EDITOR_TERRAIN_KIND` (`wall` | `cover` | `slab` |
//! `emplacement`, case-insensitive) — pre-selects the TERRAIN form's kind segment on the live
//! [`TerrainDraft`](crate::terrain_form::TerrainDraft) before the settle / screenshot, so QA can
//! capture a SPECIFIC kind's authoring surface. For `emplacement` it ALSO pre-selects the FIRST
//! (sorted) [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) key as the mounted weapon
//! when none is selected — so the shot shows the registry-backed mounted-weapon dropdown POPULATED
//! with a real weapon (the AC-7 positive content). Honored only when the capture affordance is
//! enabled; egui closures never run headless, which is why this drive exists. Example:
//! `GDTF_EDITOR_SHOT=/abs/emplacement.png GDTF_EDITOR_MODE=terrain GDTF_EDITOR_TERRAIN_KIND=emplacement cargo run -p gdtf_content_editor_bin`.
//!
//! ## GTW-669: ATTACHMENT named-item pre-load
//!
//! A further optional env var — `GDTF_EDITOR_ATTACHMENT` (a registry key / file stem, e.g.
//! `scoped_sight`) — pre-loads the NAMED attachment item into the live
//! [`AttachmentDraft`](crate::AttachmentDraft) through the SAME `load_attachment` path the
//! ATTACHMENT mode's load `ComboBox` commits, so QA can capture a SPECIFIC item's slot +
//! effect rows (the GTW-574 terrain-kind precedent — egui combo closures never run headless).
//! Unset (or a key that resolves nothing) keeps the mode's sorted-first autoload. Honored only
//! when the capture affordance is enabled. Example:
//! `GDTF_EDITOR_SHOT=/abs/attachment.png GDTF_EDITOR_MODE=attachment GDTF_EDITOR_ATTACHMENT=scoped_sight cargo run -p gdtf_content_editor_bin`.
//!
//! ## GTW-515 C4.11: PREFAB capture + zoom-applied variant
//!
//! With `GDTF_EDITOR_MODE=prefab` the capture drives the PREFAB scenario deterministically: it
//! shrinks the grid to [`SHOT_GRID_EDGE`](drive::stage::SHOT_GRID_EDGE)²×1, selects the first
//! distinct palette tile, paints
//! a block, and hovers a legal cell beside it — so the render-to-texture viewport shows a painted
//! block + the hover ghost. A further optional env var — `GDTF_EDITOR_ZOOM` (a float in
//! `[0.25, 4.0]`) — forces a non-`1.0` preview zoom before the shot, so a SECOND capture proves
//! the PROJECTION-SCALE path renders (catches an empty/black viewport at a scaled projection);
//! `GDTF_EDITOR_VIEW=full` forces the GTW-532 full-view variant and `GDTF_EDITOR_VIEW=isolate`
//! the GTW-594 three-class Isolate variant (edit storey lifted to the painted upper storey).
//!
//! ## GTW-510: capture CORE delegated to `gdtf_screenshot`
//!
//! The capture CORE — the settle-frame counter, the screenshot spawn, and the poll-then-exit — is
//! the reusable `gdtf_screenshot` crate (GTW-510), shared with the game. This module DELEGATES
//! those steps to [`settle_then_capture`](gdtf_screenshot::settle_then_capture) /
//! [`poll_then_exit`](gdtf_screenshot::poll_then_exit) keyed off the crate's
//! [`CapturePath`](gdtf_screenshot::CapturePath) /
//! [`CaptureProgress`](gdtf_screenshot::CaptureProgress) /
//! [`SettleFrames`](gdtf_screenshot::SettleFrames) / [`PollCap`](gdtf_screenshot::PollCap)
//! resources — the editor keeps ONLY its editor-specific DRIVE.
//!
//! ## GTW-512 C1.5: re-pointed onto the model
//!
//! The drive mutates the MODEL resources directly — exactly the state a real click produces —
//! which the egui shell + the (C4) viewport then render: the [`MapEditorSession`](crate::session::MapEditorSession)
//! grid size, the selected palette tile, a painted block in the [`EditorMap`](crate::EditorMap),
//! and the hovered cell via [`HoveredCell`](crate::HoveredCell).
//!
//! Split by concern (GTW-574; the single `capture.rs` outgrew the file caps): [`plugin`] (the
//! env-read plugin + registration), [`forced`] (the parsed force-value newtypes), [`drive`] (the
//! per-frame force + model-drive systems). Wiring-only here.

mod drive;
mod forced;
mod plugin;

pub use plugin::EditorCapturePlugin;
