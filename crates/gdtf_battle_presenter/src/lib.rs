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
//! [`mod@render::world_camera`]: the [`WorldCamera`]-marked `Camera2d` spawned/despawned on the
//! `GameState::BattleScape` boundary, rendering beneath the GTW-120 UI camera on its own
//! [`WORLD_RENDER_LAYER`]. It adds no sprite draw, atlas, or sim read.
//!
//! GTW-217 (the S3 slice) renames the renderer surface to `TopDown*`
//! (the dead CP437 8×8-glyph approach is replaced by the landed role-separated 16×16
//! top-down sprite set, GTW-224) and stands up the px/coordinate bridge in
//! [`mod@render::topdown`]: the [`CELL_PX`] cell-size const, the [`cell_to_world`] sim→view
//! projection, and the role-keyed [`TopDownAtlases`] resource loaded ONCE from the
//! three render sheets (terrain / characters / effects) by [`TopDownRendererPlugin`].
//! It still spawns NO sprite and draws NOTHING — that is S4/S5/S6.
//!
//! GTW-218 (the S4 slice) adds the first VISUAL draw in [`mod@render::terrain`]: the static
//! battlefield drawn as 16x16 terrain sprites from the three sim-owned static-map
//! resources for the presenter-owned [`ActiveLevel`], resolving each tile's pixels
//! from its graphic name's SPRITE DEF (GTW-665 — the
//! [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry) loaded from
//! `assets/content/sprites/*.spritedef.ron` by the HOST's Load pass; the presenter's
//! [`resolve_sprite`] is the ONE resolution both the battle draw and the content editor
//! consume). The
//! [`TopDownRendererPlugin`] inserts the [`ActiveLevel`]
//! default, defines the [`PresenterSystems::Draw`] set after
//! `SimSystems::Simulate`, and registers the one-shot [`draw_static_battlefield`] +
//! the [`swap_destroyed_cover`] reaction (both gated on the sim's `BattleInProgress`).
//! It draws NO gangers (S5), NO FX (S6), and reads NO input (S7/S8).
//!
//! GTW-342 (the squad fog WRITER, leaf 6 of the GTW-13 FOV epic) adds [`mod@actors::fog`]: the
//! [`present_fog`] system MODULATES the already-drawn layer from the sim's
//! [`SquadVisibility`](gdtf_battle_sim::visibility::SquadVisibility) — terrain VISIBLE → full colour /
//! EXPLORED → full-brightness GREYSCALE (GTW-348 — colour-loss as the memory cue, not
//! brightness-loss) / UNSEEN → hidden. Each actor sprite is hard-cut by
//! [`is_ganger_visible`](gdtf_battle_sim::visibility::is_ganger_visible) (a player ganger always shown,
//! an enemy / corpse shown iff its cell is squad-VISIBLE) inside the GTW-627
//! ganger-visibility resolver ([`resolve_ganger_visibility`]), the one writer of every
//! ganger sprite's `Visibility`. The terrain renders through a
//! [`TerrainFogMaterial`] (a [`Material2d`](bevy::sprite_render::Material2d) with a `saturation`
//! knob the fog writer drives per cell: `1.0` VISIBLE colour, `0.0` EXPLORED greyscale),
//! because the [`Sprite`](bevy::prelude::Sprite) pipeline's per-channel multiply tint cannot
//! desaturate (GTW-348); gangers stay on the sprite path. It mutates the existing material /
//! sprites in place (never despawn + respawn) and is ordered `.after`
//! [`draw_static_battlefield`] / [`swap_destroyed_cover`] so it always colours the LIVE
//! terrain, even after an [`ActiveLevel`] cycle. The sim owns the fog; this is the VIEW
//! that mirrors it (the public [`present_fog`] seam). It mints NO fire / targeting
//! fog-gate UX (GTW-11).

mod plugin;

/// Dynamic per-entity view layer: character sprites, fog-of-war view, transient combat FX.
pub mod actors;
/// Input-bridge overlay seam: highlight, path preview, fire target, and the shared targeting gate.
pub mod overlays;
/// The presenter's own clock over the sim's act log: the playback cursor, the drawn-state
/// mirrors it writes, and the catch-up predicate the input gate keys on (GTW-727).
pub mod playback;
/// Static rendering foundation: top-down projection/atlases, world camera, terrain draw.
pub mod render;

// Crate-root module re-export so intra-crate `crate::fx::` sub-path references in
// tests (e.g. `actors/fx/impact.rs`) keep resolving after the `fx` module moved
// from the crate root into `actors/`.
pub use actors::{
    fog::{
        Brightness, ShownSquadVisibility, TerrainFogMaterial, TerrainFogUniform, present_fog,
        promote_shown_fog,
    },
    fx,
    fx::{
        ArmorBrokenFct, BleedingFct, COMPASS_DIRECTIONS, CombatLogEvent, CombatLogSource,
        CombatLogSourceAppExt, CombatLogSystems, CombatText, ConsequenceFct, ConsequenceFctAppExt,
        ConsequenceFctSystems, ConsequencePop, DIRECTION_COUNT, DamageTypeFx, DotFct, EffectRoles,
        FctEmphasis, FctRiseRate, FctStackCounter, FctStackIndex, FctTtlSeconds, FctValence,
        FieldFct, FlashTtl, FloatingCombatText, FxFlash, FxReaderAppExt, FxTuning,
        IMPACT_FRAME_COUNT, ImpactFrameSeconds, InjuryFct, InjuryLogText, InterShotSeconds,
        LogLine, LogName, OnDeathFct, PendingImpact, PopAnchor, ProjectileDrawScale,
        ProjectileTravel, ProjectileVelocity, ShotImpactResolved, ShotProjectile, SuppressionFct,
        advance_projectiles, animate_floating_text, animate_impact, classify_log_event,
        expire_flashes, forward_live_log_source, forward_log_source, forward_turn_started,
        nearest_direction_index, read_armor_broken, read_bleeding, read_consequence_fct,
        read_cover_destroyed, read_fall_occurred, read_melee_resolved, read_throw_resolved,
        register_consequence_fct_core, reset_fct_stacks, severity_color, spawn_shot_projectiles,
        valence_color,
    },
    ganger::{
        CharacterRoles, FacingFrame, GangerSprite, GangerSprites, GangerVisibilityFacts,
        SpriteTween, advance_sprite_tweens, despawn_killed_ganger_on_impact,
        despawn_removed_ganger_sprites, facing_frame, move_ganger_sprites,
        resolve_ganger_appearance, resolve_ganger_visibility, spawn_ganger_sprites,
        update_ganger_life_state,
    },
};
// GTW-450 — the reachable-range overlay is the DEBUG-only overlay: every public item
// (the read-seam, the flag, the draw system) compiles only under `#[cfg(debug_assertions)]`
// (C1), so the re-export is debug-gated too — in release nothing references these.
#[cfg(debug_assertions)]
pub use overlays::reachable::{
    REACHABLE_OVERLAY_ENV, ReachableCellSprite, ReachableCells, ReachableOverlayEnabled,
    draw_reachable_overlay,
};
pub use overlays::{
    cross_level_signals::{
        BADGE_CAP_PER_CELL, CrossLevelBadgeKind, CrossLevelBadgeLabel, CrossLevelBadgeTile,
        CrossLevelSignals, CrossLevelSimFacts, LevelDelta, ThreatCount, derive_cross_level_signals,
        draw_cross_level_signals,
    },
    field::{FieldCellSprite, draw_field_overlay},
    fire_target::{FireTargetHighlight, FireTargetLabel, FireTargetTile, draw_fire_target},
    highlight::{HighlightRequest, HoverHighlight, draw_highlight_on_request},
    path_preview::{PathPreview, PathStepSprite, PathTargetLabel, draw_path_preview},
    targeting_gate::{CellVisibility, cell_squad_visible},
};
pub use playback::{
    ActHold, ActHoldPhase, ConsequenceSeconds, DrawnLife, DrawnMagazine, DrawnPose, DrawnPosition,
    DrawnVitals, DrawnWriters, FireBeatSeconds, FxPipelineProbe, FxSeenBusy, ImpactCapSeconds,
    LifeChangeSeconds, MinorSeconds, PlaybackCursor, PlaybackGate, PlaybackTuning, Played,
    PlayedSignals, PostureSeconds, ReactionBeatSeconds, ReloadSeconds, RoundSeconds, SkippedActs,
    StepSeconds, TurnBeatSeconds, advance_playback, playback_caught_up, register_playback,
    seed_drawn_state,
};
pub use plugin::{
    BattlePresenterMode, BattlePresenterPlugin, IsoRendererPlugin, TopDownRendererActive,
    TopDownRendererPlugin,
};
pub use render::{
    terrain::{
        ActiveLevel, ContextDepth, IsolateView, MissingTileTexture, PresenterSystems,
        SpriteResolveCtx, StampedGraphic, StaticMap, StoreyTreatment, StoreyViewMode,
        TerrainSprite, TileRole, VerticalLinkSprite, ViewMode, anchor_world_offset,
        draw_static_battlefield, draw_vertical_links, indicate_emplacement_occupied,
        resolve_sprite, restamp_tiles_on_def_change, setup_missing_tile_texture,
        single_rect_layout, source_parts, source_px_size, source_urect, storey_treatment,
        swap_destroyed_cover, swap_destroyed_slab,
    },
    topdown::{
        CELL_PX, GANGER_Z_BIAS, Layer, SheetAtlas, SheetRole, TileIndex, TopDownAtlases,
        cell_to_world, cell_to_world_layered, load_topdown_atlases,
        redrive_sheet_images_on_asset_event, sim_pos_to_world,
    },
    world_camera::{
        BoundsMarginWorld, DwellDelaySeconds, DwellElapsed, EdgeBandPx, GamepadCursorMoved,
        PanEdgeDwellState, PanSpeed, PanTuning, STICK_DEADZONE, StickDeadzone, WORLD_RENDER_LAYER,
        WorldCamera, camera_focus, clamp_camera, clamp_camera_to_bounds, despawn_world_camera,
        frame_camera_on_units, keyboard_pan_dir, mouse_edge_dir, pan_camera,
        pan_camera_on_gamepad_cursor_edge, pan_velocity, should_edge_pan_after_dwell,
        spawn_world_camera, stick_pan_dir, viewport_edge_dir,
    },
};
