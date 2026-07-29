//! The GDTF **map editor** — a SEPARATE windowed binary from the game (GTW-417).
//!
//! This crate is the FOUNDATION of the GTW-404 map-editor track: an app shell that launches a
//! windowed editor with the theme + content registries loaded. It mirrors `gdtf_app`'s shape — a
//! [`MapEditorApp`] wrapper over a Bevy `App` — but runs its OWN minimal [`EditorState`] machine and
//! shares NONE of the game's scene graph or battle sim (the housing constraint: the procgen assembly
//! + its dev-tools load-time stepper live in the main game, not the editor).
//!
//! ## GTW-512: the egui Workbench shell (C1 of the GTW-511 migration)
//!
//! The editor's UI is `bevy_egui` (GTW-512) — a CLEAN SWAP off the hand-rolled `bevy_ui` shell (the
//! four `bevy_ui` regions + the segmented-control mode tabs + the dropdown/numeric-field widgets +
//! the palette / canvas / right-panel drive systems are GONE). The egui shell draws the WHOLE
//! editor in ONE system in the [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) schedule
//! (mode tabs + global theme `ComboBox`, a status line, a palette/stats placeholder, the active mode's
//! form, and a viewport placeholder), reading the kept MODEL resources. C1 is the SHELL + the capture
//! re-point; the three per-mode FORMS are stubbed minimally (the full TERRAIN / THEME / PREFAB forms +
//! the texture viewport are the later children C2 / C3 / C4).
//!
//! - [`MapEditorApp`] composes `DefaultPlugins` + [`EguiPlugin`](bevy_egui::EguiPlugin) +
//!   [`MapEditorPlugin`] + the env-gated QA capture affordance.
//! - [`MapEditorPlugin`] wires the [`EditorState`] machine, the slim `Load` asset pass, the
//!   [`Editing`](EditorState::Editing) scene's state-scoped model lifecycle + the standalone camera,
//!   and the egui shell UI system.
//! - [`EditorCapturePlugin`] is the OFF-by-default QA hook for the screenshot (re-pointed in GTW-512
//!   to drive the MODEL resources directly + the new [`HoveredCell`], so it stays QA-able under egui).
//! - [`MapEditorSession`] is the shared theme/default-floor/grid-size/selected-tile selection state
//!   the egui shell writes and the (kept) model reads. Swept onto the UUID model (GTW-495): the theme
//!   is a `ThemeUuid` and the default-floor / paint tile are `TerrainUuid`s.
//! - [`EditorMode`] is the state-scoped active-mode resource the egui mode tabs + the `1`/`2`/`3`
//!   hotkeys (`mode`) drive; the egui shell branches its right panel on it.
//! - [`HoveredCell`] is the hovered-cell MODEL the live egui hover (C4) + the QA capture both write,
//!   so the preview ghost is QA-able headlessly (GTW-512 C1.5).
//! - The `canvas` module keeps the two MODEL resources the editor's lifecycle inserts —
//!   [`CurrentEditLevel`] (the storey selector — GTW-500 C1) + [`CanvasZoom`] (the viewport zoom —
//!   GTW-500 C3, reused as the preview camera's `OrthographicProjection::scale`); the egui viewport
//!   (GTW-515 C4) reads them. The `bevy_ui` cell-grid render is gone.
//! - The GTW-515 `preview` module owns the render-to-texture VIEWPORT (C4.3): an offscreen render
//!   target [`Image`](bevy::image::Image), a dedicated second [`Camera2d`](bevy::prelude::Camera2d)
//!   rendering the prefab preview tiles into it on an ISOLATED
//!   [`RenderLayers`](bevy::camera::visibility::RenderLayers), the change-driven tile redraw + hover
//!   ghost, and the once-per-frame set-to-target zoom/pan apply. The egui PREFAB mode (the
//!   `egui_shell::prefab` submodule) draws that registered image as its central-panel viewport,
//!   folding click / wheel / drag input into the [`EditorMap`] + [`CanvasZoom`] + [`PreviewPan`].
//! - The GTW-426 `editor_map` module owns [`EditorMap`] — the in-memory, state-scoped paintable map
//!   model (a sparse `CellLevel → TerrainUuid` store of painted cells, level-aware since GTW-430,
//!   UUID-keyed since GTW-495). The authoritative record the paint flow writes + the save path reads.
//! - The GTW-430 `placement` module owns the SINGLE SHARED placement-legality predicate
//!   ([`evaluate_placement`]) the preview + the commit run, plus the multi-level auto-handling
//!   ([`PlacementVerdict`] / [`ProposedPlacement`]).
//! - The GTW-432 `save` module (debug-only) keeps the pure PROJECTION of the [`EditorMap`] into the
//!   [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec) schema + the RON serialize + the path resolution (the egui save controls + the
//!   fs-write press are deferred to the C4 child); the projection re-checks every painted cell through
//!   [`evaluate_placement`] so a saved prefab never contains an illegal cell.
//! - The `right_panel` module keeps the LIVE size-field value newtype ([`GridSpanInput`] — the
//!   GTW-464 [`SizeFieldSpans`] view model is built over it) + `seed_default_theme`;
//!   the dead `bevy_ui` marker types (`SizeFieldAxis` / `ThemeDropdown`) were deleted in GTW-577 C7.
//! - The GTW-495 `terrain_graphics` module resolves a `TerrainDef`'s `presenter_kind.graphic_name`
//!   to its SPRITE DEF the way the presenter does (through the presenter's `resolve_sprite` over
//!   the GTW-663 sprite-def registry — GTW-665).

mod app;
// GTW-479: the ARMOR authoring mode of the Workbench — the form that edits an armor
// suit's six per-body-part pieces and saves it to `content/armor/<name>.armor.ron`
// where the GTW-269 armor loader reads (the GTW-636 Gang mode/form precedent).
mod armor_form;
// GTW-669: the ATTACHMENT authoring mode of the Workbench — the form that edits an
// attachment item (display name / the closed 6-slot mount / the closed 13-effect list)
// and saves it to `content/attachments/<name>.attachment.ron` where the GTW-619
// attachments loader reads (the GTW-479 Armor mode/form precedent).
mod attachment_form;
// GTW-512 C1.2: the editor's standalone 2D camera (the `bevy_ui` shell used to spawn it).
mod camera;
mod canvas;
mod capture;
// GTW-531: the prefab-editor vertical-connector auto-pairing — placing an UP connector at (x,y,N)
// also places its paired DOWN connector at (x,y,N+1). REUSES the shared placement predicate; a
// prefab-editor placement rule ONLY (no sim/runtime change).
mod connector_pairing;
// GTW-512 C1: the egui Workbench shell — the CLEAN SWAP off the hand-rolled `bevy_ui` shell.
mod editor_map;
mod egui_shell;
// GTW-636: the GANG authoring mode of the Workbench — the form that edits a gang roster
// and saves it to `content/gangs/<name>.gang.ron` where the GTW-415 gangs loader reads
// (the USER RULING 2026-07-06: gangs are authored OUTSIDE the game binary; this mode
// replaces the retired in-game gang editor at full parity).
mod gang_form;
// GTW-512 C1.5: the hovered-cell model the live egui hover + the QA capture both write.
mod hovered_cell;
// GTW-654: the INJURY authoring mode of the Workbench — the forms that edit an injury
// def (`<category>/<key>.injury.ron`, incl. the closed-palette effects list) and a
// per-category weighting table (`weighting/<category>.weighting.ron`), saving both
// where the bespoke GTW-437 injuries folder loader reads (the GTW-479 Armor
// mode/form precedent).
mod injury_form;
mod load;
// GTW-671: the MELEE-WEAPON authoring mode of the Workbench — the form that edits a
// full MeleeWeaponSpec (the shared damage group, reach, fight modes, shove, slots,
// attachments) and saves it to `content/weapons/melee/<name>.melee_weapon.ron` where
// the GTW-505/570 MeleeWeaponsFamily loader reads (the GTW-670 Weapon mode/form
// precedent — the last GTW-478 child).
mod melee_weapon_form;
// GTW-474: the Workbench mode machine (the EditorMode resource) — GTW-512 trimmed it to the enum +
// the `1`/`2`/`3` hotkeys (the egui shell draws the tabs + branches the right panel in-UI).
mod mode;
// GTW-804 (child 1b of GTW-786): the DEV-ONLY QA network control channel for the EDITOR — a
// loopback listener on the SHARED gdtf_net_qa_transport (GTW-803) that a coding-agent QA client
// drives. DOUBLE-gated: the module compiles only in a debug build WITH the opt-in `net_qa`
// feature (it opens a TCP listener, so a release editor never sees it), and even then it stays
// inert until `GDTF_EDITOR_NET_QA` is set truthy. This child wires Hello/version negotiation
// only.
#[cfg(all(debug_assertions, feature = "net_qa"))]
mod net_qa;
mod placement;
mod plugin;
// GTW-515 C4.3: the prefab preview RENDER MACHINERY — an offscreen render-target image, a dedicated
// second Camera2d on an isolated RenderLayers that renders the prefab preview tiles into it, and the
// change-driven tile redraw + the set-to-target zoom/pan apply. The egui PREFAB mode draws this
// registered image as its central-panel viewport.
mod preview;
// GTW-421 `GridSpanInput` (LIVE — the GTW-464 size-field view model builds on it) + the
// `seed_default_theme` drive, kept across the egui swap (GTW-512); the `bevy_ui` spawn, the
// hand-rolled-widget commit drives (GTW-512), and the dead `SizeFieldAxis` / `ThemeDropdown`
// markers (GTW-577 C7) were dropped.
mod right_panel;
// The GTW-432 save-prefab path is debug-only (the GTW-429 gang-save precedent): the whole module
// — the EditorMap → PrefabSpec projection, the RON serialize, the fs-write, and the press trigger
// — is gated `#[cfg(debug_assertions)]` so it never compiles into a release binary.
#[cfg(debug_assertions)]
mod save;
mod session;
// GTW-664: the SPRITE authoring mode of the Workbench — the form that edits a sprite def
// (source / anchor / optional facings / optional animation — the GTW-600 ruled schema)
// and saves it to `content/sprites/<name>.spritedef.ron` where the GTW-663
// SpriteDefsFamily loader reads (the GTW-479 Armor mode/form precedent).
mod sprite_form;
mod state;
// GTW-474: the TERRAIN authoring mode of the Workbench — the form that captures a TerrainDef and
// saves it to a per-theme `.terrain_def.ron` the GTW-487 loader resolves.
mod terrain_form;
// GTW-475: the THEME authoring mode of the Workbench — the form that assembles a UuidThemeDef BY
// REFERENCE (terrain UUIDs + a default floor) and saves it to a per-theme `.terrain_theme.ron`
// the GTW-487 theme loader resolves.
mod theme_form;
// GTW-495: resolve a TerrainDef's presenter_kind.graphic_name to its sprite def THE WAY
// THE PRESENTER DOES (via the presenter's resolve_sprite over the sprite-def registry —
// GTW-665) — shared by the palette + preview.
mod terrain_graphics;
// GTW-634 C2: the ONE per-theme directory naming policy (slug + `unknown_theme` fallback)
// the terrain-def + prefab savers share — the theme form's `slugify` stays a DELIBERATELY
// divergent sibling (empty → error, no fallback).
mod theme_dir;
// GTW-630: the editor's AUTHORING-TIME registration of the GTW-582 reference-integrity pass —
// the shared gdtf_content_families::validate checks over the edges the editor loads
// (theme→terrain + emplacement→weapon + gang equipment — GTW-651), re-armed live on
// hot-reload.
mod validate;
// GTW-670: the WEAPON authoring mode of the Workbench — the form that edits a full
// ranged WeaponSpec (all 18 authored fields) and saves it to
// `content/weapons/ranged/<name>.weapon.ron` where the GTW-257/570 WeaponsFamily
// loader reads (the GTW-479 Armor mode/form precedent).
mod weapon_form;

pub use app::MapEditorApp;
// GTW-479: the ARMOR-mode model + the pure save halves — exported so the round-trip
// tests drive the REAL projection / path resolution / write (the TempDir round-trip
// through the actual ArmorFamily loader) and the headless lifecycle test asserts the
// scoped draft (the GTW-636 gang export precedent).
pub use armor_form::{ArmorDraft, armor_file_name, armor_save_path_in, draft_to_spec};
// The debug-only ARMOR fs-write surface (the terrain/theme/gang write precedent):
// `write_armor_in` is the root-parameterized core tests aim at a `TempDir`;
// `write_armor` is the production wrapper (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use armor_form::{write_armor, write_armor_in};
// GTW-669: the ATTACHMENT-mode model + the pure save halves — exported so the
// round-trip tests drive the REAL projection / path resolution / write (the TempDir
// round-trip through the actual AttachmentsFamily loader) and the headless lifecycle
// test asserts the scoped draft (the GTW-636/GTW-479 export precedent).
pub use attachment_form::{
    AttachmentDraft, attachment_file_name, attachment_save_path_in, draft_to_attachment_spec,
};
// The debug-only ATTACHMENT fs-write surface (the terrain/theme/gang/armor/sprite write
// precedent): `write_attachment_in` is the root-parameterized core tests aim at a
// `TempDir`; `write_attachment` is the production wrapper (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use attachment_form::{write_attachment, write_attachment_in};
// GTW-512: the `bevy_ui` canvas render markers are GONE (the egui viewport is C4); only the two
// model resources the editor's lifecycle inserts survive (the egui viewport reads them in C4).
pub use canvas::{CanvasZoom, CurrentEditLevel, LevelStep};
pub use capture::EditorCapturePlugin;
// GTW-531: the up→down connector auto-pairing surface — exported so the prefab viewport commit
// (and the in-crate + integration round-trip tests) drive the real path.
pub use connector_pairing::{
    PairingOutcome, apply_placement_with_pairing, is_up_connector, resolve_down_counterpart,
};
pub use editor_map::EditorMap;
// GTW-464: the PREFAB size fields' view model — displayed spans derived FRESH from the session
// every egui pass (the session → fields reverse sync) + the kept clamp commit. Exported so the
// headless test asserts the exact model the panel renders from (the GTW-512 pattern).
pub use egui_shell::prefab::size_fields::SizeFieldSpans;
// GTW-636: the GANG-mode model + the pure save halves — exported so the C5 tests drive
// the REAL projection / path resolution / write (the TempDir round-trip through the
// actual GangsFamily loader) and the headless lifecycle test asserts the scoped draft.
pub use gang_form::{GangDraft, draft_to_roster, gang_file_name, gang_save_path_in};
// The debug-only GANG fs-write surface (the terrain/theme write precedent):
// `write_gang_in` is the root-parameterized core tests aim at a `TempDir`; `write_gang`
// is the production wrapper (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use gang_form::{write_gang, write_gang_in};
pub use hovered_cell::HoveredCell;
// GTW-654: the INJURY-mode models + the pure save halves — exported so the round-trip
// tests drive the REAL projections / path resolutions / writes (the TempDir round-trip
// through the actual bespoke injuries loader) and the headless lifecycle test asserts
// the scoped drafts (the GTW-636/GTW-479 export precedent).
pub use injury_form::{
    InjuryDraft, WeightingDraft, draft_to_def, draft_to_weighting, injury_file_name,
    injury_save_path_in, weighting_file_name, weighting_save_path_in,
};
// The debug-only INJURY fs-write surface (the terrain/theme/gang/armor write
// precedent): the `*_in` cores are root-parameterized for `TempDir` tests; the bare
// wrappers are the production Save-button paths (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use injury_form::{write_injury, write_injury_in, write_weighting, write_weighting_in};
// GTW-671: the MELEE-WEAPON-mode model + the pure save halves — exported so the
// round-trip tests drive the REAL projection / path resolution / write (the TempDir
// round-trip through the actual MeleeWeaponsFamily loader) and the headless lifecycle
// test asserts the scoped draft (the GTW-636/GTW-479/GTW-670 export precedent).
pub use melee_weapon_form::{
    MeleeWeaponDraft, draft_to_melee_weapon_spec, melee_weapon_file_name, melee_weapon_save_path_in,
};
// The debug-only MELEE-WEAPON fs-write surface (the terrain/theme/gang/armor/sprite/
// attachment/weapon write precedent): `write_melee_weapon_in` is the root-parameterized
// core tests aim at a `TempDir`; `write_melee_weapon` is the production wrapper
// (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use melee_weapon_form::{write_melee_weapon, write_melee_weapon_in};
// GTW-512: only the `EditorMode` enum survives the egui swap (the `bevy_ui` tab / content markers
// are gone — the egui shell draws the tabs + branches the right panel in-UI).
pub use mode::EditorMode;
// GTW-804: the editor's DEV QA channel surface, exported under the SAME double gate the module
// carries — the plugin the binary wires (and the integration test drives against a real bound
// listener), its request-drain system set, and the server name the handshake reports.
// GTW-880 adds the capture pump's four tunables to that surface: the confinement directory,
// the settle window, the poll budget and which pixels a capture reads — so the integration
// suite can pin a temp directory, a short settle and an offscreen source.
#[cfg(all(debug_assertions, feature = "net_qa"))]
pub use net_qa::{
    EDITOR_QA_SERVER_NAME, EditorNetQaSystems, EditorQaShotDir, EditorShotPollBudget,
    EditorShotSettle, EditorShotSource, NetQaEditorPlugin,
};
pub use placement::{
    EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement, classify,
    evaluate_placement, names_a_ladder,
};
pub use plugin::MapEditorPlugin;
// GTW-515: the prefab preview render-target resource + the owned pan-offset target (the zoom target
// is the kept `CanvasZoom`). Exported so the headless test asserts the state-scoped lifecycle.
pub use preview::{target::PreviewTarget, view::PreviewPan};
pub use right_panel::GridSpanInput;
// GTW-512: the save PROJECTION surface (debug-only, the v2 save path) — kept for the C4 save-control
// re-point + the in-crate save tests. The `bevy_ui` save controls themselves are the C4 child.
// GTW-662: `write_prefab_in` / `prefab_save_path_in` are the root-parameterized cores — the
// round-trip test aims them at a `tempfile::TempDir` root so it never writes into the
// version-controlled `assets/` tree; `write_prefab` / `prefab_save_path` are the production
// wrappers (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use save::{
    SavePrefabError, editor_map_to_prefab, prefab_save_path, prefab_save_path_in, sanitize_name,
    serialize_prefab, write_prefab, write_prefab_in,
};
pub use session::MapEditorSession;
// GTW-664: the SPRITE-mode model + the pure save halves — exported so the round-trip
// tests drive the REAL projection / path resolution / write (the TempDir round-trip
// through the actual SpriteDefsFamily loader) and the headless lifecycle test asserts
// the scoped draft (the GTW-636/GTW-479 export precedent).
pub use sprite_form::{SpriteDraft, draft_to_sprite_def, sprite_file_name, sprite_save_path_in};
// The debug-only SPRITE fs-write surface (the terrain/theme/gang/armor write precedent):
// `write_sprite_in` is the root-parameterized core tests aim at a `TempDir`;
// `write_sprite` is the production wrapper (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use sprite_form::{write_sprite, write_sprite_in};
pub use state::EditorState;
// GTW-566 C5: `TerrainGraphicChoice` is GONE — the graphic pick is the presenter's
// `TileRole` vocabulary directly, filtered through `offered_graphic_roles`.
pub use terrain_form::{
    ArmorInput, FootfallChoice, HpInput, SaveTerrainError, TerrainDraft, TerrainKindChoice,
    draft_to_terrain_def, offered_graphic_roles, serialize_terrain_def,
};
// The debug-only TERRAIN / THEME fs-write surface — kept for the C2 / C3 egui save-press re-point.
// `write_terrain_in` is the root-parameterized core: tests call it with a `tempfile::TempDir` root
// so they never write into the version-controlled `assets/` tree. `write_terrain` is the production
// thin wrapper (WORKSPACE_ASSETS_ROOT). Both are `cfg(debug_assertions)`-only.
#[cfg(debug_assertions)]
pub use terrain_form::{write_terrain, write_terrain_in};
pub use theme_form::{
    SaveThemeError, ThemeDraft, draft_to_theme_def, floor_candidates, resolved_stats,
    serialize_theme_def, slab_floor_candidates, validate_for_save,
};
// The debug-only THEME fs-write surface (the terrain/gang/armor/sprite/weapon write precedent):
// `write_theme_in` is the root-parameterized core tests aim at a `TempDir` (GTW-662);
// `write_theme` is the production wrapper (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use theme_form::{write_theme, write_theme_in};
// GTW-670: the WEAPON-mode model + the pure save halves — exported so the round-trip
// tests drive the REAL projection / path resolution / write (the TempDir round-trip
// through the actual WeaponsFamily loader) and the headless lifecycle test asserts the
// scoped draft (the GTW-636/GTW-479 export precedent).
pub use weapon_form::{WeaponDraft, draft_to_weapon_spec, weapon_file_name, weapon_save_path_in};
// The debug-only WEAPON fs-write surface (the terrain/theme/gang/armor/sprite/
// attachment write precedent): `write_weapon_in` is the root-parameterized core tests
// aim at a `TempDir`; `write_weapon` is the production wrapper (WORKSPACE_ASSETS_ROOT).
#[cfg(debug_assertions)]
pub use weapon_form::{write_weapon, write_weapon_in};
