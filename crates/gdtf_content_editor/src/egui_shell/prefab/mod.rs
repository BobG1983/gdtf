//! The egui PREFAB-mode authoring form (GTW-515 C4) — the real prefab painter that replaces the
//! C1 stub across the shell's LEFT palette / CENTRAL viewport / RIGHT controls panels.
//!
//! This module is the egui DRAW + input for the PREFAB mode. It REUSES the prefab model + save +
//! placement layer VERBATIM (C4.10): the [`EditorMap`](crate::EditorMap) paintable model, the
//! shared [`evaluate_placement`](crate::evaluate_placement) / [`apply_placement`](crate::apply_placement)
//! legality (ladder auto-clear, slab-seals-ladder reject), the presenter-mirroring
//! [`terrain_atlas_index`](crate::terrain_graphics) sprite resolution, and the debug-only
//! [`write_prefab`](crate::save::write_prefab) save. The RENDER-TO-TEXTURE viewport machinery (the
//! offscreen image + the dedicated isolated-render-layer camera + the change-driven tile redraw +
//! the set-to-target zoom/pan apply) lives in [`preview`](crate::preview); this module only DRAWS
//! the registered preview image inside the egui central panel and folds the pointer / wheel / drag
//! input into the owned model resources.
//!
//! ## Layout across the C1 shell panels (mirrors C2 / C3)
//!
//! - LEFT palette panel — [`palette_ui::palette_panel`]: one selectable sprite row per theme
//!   terrain + a selected-tile stat summary (C4.2).
//! - CENTRAL viewport panel — [`viewport_ui::viewport_panel`]: the render-to-texture
//!   [`egui::Image`](bevy_egui::egui::Image) + click-to-paint + hover ghost + wheel-zoom +
//!   right-drag pan (C4.3 / C4.4 / C4.7 / C4.8).
//! - RIGHT controls panel — [`controls_ui::controls_panel`]: the grid-size fields (drawn +
//!   two-way-synced by [`size_fields`] over its [`SizeFieldSpans`](size_fields::SizeFieldSpans)
//!   view model — GTW-464), the per-storey LEVEL RAIL ([`level_rail`] — GTW-595, replacing the
//!   blind `Level n / m` paging), and the debug-only Save-prefab control (C4.5 / C4.6 / C4.9).
//!
//! The `]`/`[`/PageUp/PageDown level-nav HOTKEYS are a UI-agnostic `Update` system
//! ([`nav::level_nav_hotkeys`]), like the kept mode hotkeys.

pub(crate) mod controls_ui;
// GTW-595: the per-storey occupancy thumbnail scrub-strip (the level rail) — the sweep /
// cache / scrub model half plus its egui rows.
pub(crate) mod level_rail;
pub(crate) mod nav;
pub(crate) mod palette_ui;
// GTW-464: the grid-size fields + their explicit SizeFieldSpans view model (session → fields
// reverse sync; the kept clamp commit).
pub(crate) mod size_fields;
pub(crate) mod viewport_ui;
