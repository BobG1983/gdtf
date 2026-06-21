//! Presentation layer for GDTFs turn based battle system.
//!
//! This is the VIEW that mirrors the authoritative, render-free combat sim
//! (`gdtf_battle_sim`). The dependency is strictly one-way: the presenter reads
//! sim state and renders it; the sim never reads the presenter.
//!
//! GTW-215 (the first GTW-48 slice) stands up the empty *home* every later slice
//! plugs into. It lands the [`BattlePresenterPlugin`] seam and the
//! [`BattlePresenterMode`] selector between the real top-down renderer
//! ([`TopDownRendererPlugin`]) and a no-op iso stub ([`IsoRendererPlugin`], a
//! placeholder for the future iso renderer, GTW-49 / GTW-10). It spawned no camera
//! (S2), loaded no atlas (S3), drew no sprite (S4/S5/S6), read no sim type by name,
//! and touched no input (S7/S8).
//!
//! GTW-216 (the S2 slice) adds the SHARED world-camera lifecycle in
//! [`mod@world_camera`]: the [`WorldCamera`]-marked `Camera2d` spawned/despawned on the
//! `GameState::BattleScape` boundary, rendering beneath the GTW-120 UI camera on its own
//! [`WORLD_RENDER_LAYER`]. It adds no sprite draw, atlas, or sim read.
//!
//! GTW-217 (the S3 slice) renames the renderer surface to `TopDown*`
//! (the dead CP437 8×8-glyph approach is replaced by the landed role-separated 16×16
//! top-down sprite set, GTW-224) and stands up the px/coordinate bridge in
//! [`mod@topdown`]: the [`CELL_PX`] cell-size const, the [`cell_to_world`] sim→view
//! projection, and the role-keyed [`TopDownAtlases`] resource loaded ONCE from the
//! three render sheets (terrain / characters / effects) by [`TopDownRendererPlugin`].
//! It still spawns NO sprite and draws NOTHING — that is S4/S5/S6.
//!
//! GTW-218 (the S4 slice) adds the first VISUAL draw in [`mod@terrain`]: the static
//! battlefield drawn as 16x16 terrain sprites from the three sim-owned static-map
//! resources for the presenter-owned [`ActiveLevel`], choosing each tile via the
//! DATA-DRIVEN [`TileRoles`] table (`assets/tiles/tile_roles.ron`). The
//! [`TopDownRendererPlugin`] loads + resolves that table, inserts the [`ActiveLevel`]
//! default, defines the [`PresenterSystems::Draw`] set after
//! `SimSystems::Simulate`, and registers the one-shot [`draw_static_battlefield`] +
//! the [`swap_destroyed_cover`] reaction (both gated on the sim's `BattleInProgress`).
//! It draws NO gangers (S5), NO FX (S6), and reads NO input (S7/S8).

mod plugin;

pub mod fx;
pub mod ganger;
pub mod highlight;
pub mod terrain;
pub mod topdown;
pub mod world_camera;

pub use fx::{
    COMPASS_DIRECTIONS, CombatLogEvent, CombatText, DIRECTION_COUNT, DamageTypeFx, EffectRoles,
    EffectRolesHandle, FctEmphasis, FctRiseRate, FctStackIndex, FctTtlSeconds, FctValence,
    FlashTtl, FloatingCombatText, FxFlash, FxTuning, FxTuningHandle, IMPACT_FRAME_COUNT,
    ImpactFrameSeconds, InterShotSeconds, LogLine, LogName, PendingImpact, ProjectileDrawScale,
    ProjectileTravel, ProjectileVelocity, ShotImpactResolved, ShotProjectile, advance_projectiles,
    animate_floating_text, animate_impact, classify_log_event, expire_flashes, load_effect_roles,
    load_fx_tuning, nearest_direction_index, read_armor_broken, read_bleeding,
    read_consequence_fct, read_cover_destroyed, redrive_fx_tuning_on_asset_event,
    resolve_effect_roles, resolve_fx_tuning, severity_color, spawn_shot_projectiles, valence_color,
};
pub use ganger::{
    CharacterRoles, CharacterRolesHandle, FacingFrame, GangerSprite, GangerSprites,
    apply_active_level_filter, despawn_killed_ganger_on_impact, despawn_removed_ganger_sprites,
    facing_frame, load_character_roles, move_ganger_sprites, reframe_ganger_sprites,
    resolve_character_roles, spawn_ganger_sprites, update_ganger_life_state,
};
pub use highlight::{HighlightRequest, HoverHighlight, draw_highlight_on_request};
pub use plugin::{
    BattlePresenterMode, BattlePresenterPlugin, IsoRendererPlugin, TopDownRendererActive,
    TopDownRendererPlugin,
};
pub use terrain::{
    ActiveLevel, PresenterSystems, StaticMap, TerrainSprite, TileIndex, TileRoles, TileRolesHandle,
    draw_static_battlefield, load_tile_roles, resolve_tile_roles, swap_destroyed_cover,
};
pub use topdown::{
    CELL_PX, GANGER_Z_BIAS, Layer, SheetAtlas, SheetRole, TopDownAtlases, cell_to_world,
    cell_to_world_layered, load_topdown_atlases, sim_pos_to_world,
};
pub use world_camera::{
    DwellDelaySeconds, DwellElapsed, EdgeBandPx, GamepadCursorMoved, PanEdgeDwellState, PanSpeed,
    PanTuning, PanTuningHandle, STICK_DEADZONE, StickDeadzone, WORLD_RENDER_LAYER, WorldCamera,
    camera_focus, clamp_camera, clamp_camera_to_bounds, despawn_world_camera,
    frame_camera_on_units, keyboard_pan_dir, load_pan_tuning, mouse_edge_dir, pan_camera,
    pan_camera_on_gamepad_cursor_edge, pan_velocity, redrive_pan_tuning_on_asset_event,
    resolve_pan_tuning, should_edge_pan_after_dwell, spawn_world_camera, stick_pan_dir,
    viewport_edge_dir,
};
