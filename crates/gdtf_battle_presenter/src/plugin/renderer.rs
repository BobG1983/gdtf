//! The presenter mode selector and the renderer plugins, with the top-down renderer's
//! full system wiring.

use bevy::{ecs::message::Messages, prelude::*, sprite_render::Material2dPlugin};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_sim::{
    ArmorBroken, BattleInProgress, Bleeding, CombatTuning, CoverDestroyed, CoverLedger,
    InjuryInflicted, OccupancyGrid, PlayerFaction, ShotFired, SlabDestroyed, SquadVisibility,
    SurfaceGrid, VerticalLinkGraph, acts::MeleeResolved, occupancy_sync::SimSystems,
};

use crate::{
    ActiveLevel, CharacterRoles, CharacterRolesHandle, EffectRoles, EffectRolesHandle,
    FireTargetHighlight, FxTuning, FxTuningHandle, GamepadCursorMoved, GangerSprites,
    HighlightRequest, PanEdgeDwellState, PanTuning, PanTuningHandle, PathPreview, PresenterSystems,
    ShotImpactResolved, TerrainFogMaterial, TileRoles, TileRolesHandle, TopDownAtlases, ViewMode,
    advance_projectiles, advance_sprite_tweens, animate_floating_text, animate_impact,
    apply_active_level_filter, clamp_camera_to_bounds, despawn_killed_ganger_on_impact,
    despawn_removed_ganger_sprites, draw_fire_target, draw_highlight_on_request, draw_path_preview,
    draw_static_battlefield, draw_vertical_links, expire_flashes, frame_camera_on_units,
    load_character_roles, load_effect_roles, load_fx_tuning, load_pan_tuning, load_tile_roles,
    load_topdown_atlases, move_ganger_sprites, pan_camera, pan_camera_on_gamepad_cursor_edge,
    present_fog, read_armor_broken, read_bleeding, read_consequence_fct, read_cover_destroyed,
    read_injury_fct, read_melee_resolved, redrive_character_roles_on_asset_event,
    redrive_effect_roles_on_asset_event, redrive_fx_tuning_on_asset_event,
    redrive_pan_tuning_on_asset_event, redrive_sheet_images_on_asset_event,
    redrive_tile_roles_on_asset_event, reframe_ganger_sprites,
    reindex_ganger_sprites_on_character_roles_change, resolve_character_roles,
    resolve_effect_roles, resolve_fx_tuning, resolve_pan_tuning, resolve_tile_roles,
    spawn_ganger_sprites, spawn_shot_projectiles, swap_destroyed_cover, swap_destroyed_slab,
    update_ganger_life_state,
};
// GTW-450 — the reachable-range overlay items are DEBUG-only (C1); imported via a
// separate `#[cfg(debug_assertions)]` `use` below so the release build never names them.
#[cfg(debug_assertions)]
use crate::{ReachableCells, ReachableOverlayEnabled, draw_reachable_overlay};

/// Which battle renderer the [`BattlePresenterPlugin`] builds.
///
/// A domain value (the named presentation mode), so it is a real type rather than
/// a bare primitive. Exhaustive: exactly the two variants the design supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePresenterMode {
    /// The top-down 16×16 sprite renderer — the shipping presenter.
    TopDown,
    /// The isometric renderer — an intentional stub for the whole of GTW-48 (the
    /// real iso renderer is GTW-49 / GTW-10).
    Iso,
}

/// The battle presenter seam: selects and builds one battle renderer per its
/// [`BattlePresenterMode`].
///
/// Added by `GameBattleScapeScenePlugin` so its `build` runs when the scene plugins
/// register. On `build` it adds the renderer plugin chosen by [`Self::mode`]:
/// [`TopDownRendererPlugin`] for [`BattlePresenterMode::TopDown`], the
/// [`IsoRendererPlugin`] stub for [`BattlePresenterMode::Iso`].
pub struct BattlePresenterPlugin {
    /// The renderer this plugin builds.
    mode: BattlePresenterMode,
}

impl BattlePresenterPlugin {
    /// Construct a presenter that builds the renderer for `mode`.
    #[must_use]
    pub const fn new(mode: BattlePresenterMode) -> Self {
        Self { mode }
    }

    /// The [`BattlePresenterMode`] this plugin builds.
    #[must_use]
    pub const fn mode(&self) -> BattlePresenterMode {
        self.mode
    }
}

impl Default for BattlePresenterPlugin {
    /// The shipping default: the top-down renderer.
    fn default() -> Self {
        Self::new(BattlePresenterMode::TopDown)
    }
}

impl Plugin for BattlePresenterPlugin {
    fn build(&self, app: &mut App) {
        match self.mode {
            BattlePresenterMode::TopDown => {
                app.add_plugins(TopDownRendererPlugin);
            }
            BattlePresenterMode::Iso => {
                app.add_plugins(IsoRendererPlugin);
            }
        }
    }
}

/// Marker resource the [`TopDownRendererPlugin`] inserts on `build`.
///
/// Its presence in the world is the test-observable proof that the top-down renderer
/// plugin's `build` actually ran (the default-mode and integration tests assert it
/// present, the iso-mode test asserts it absent). A framework type (`Resource`), so
/// it is exempt from the no-bare-types rule.
#[derive(Resource)]
pub struct TopDownRendererActive;

/// The real top-down 16×16 sprite battle renderer plugin.
///
/// On `build` it inserts the [`TopDownRendererActive`] marker and schedules
/// [`load_topdown_atlases`] in [`Startup`] so the role-keyed [`TopDownAtlases`]
/// resource is built ONCE and present before the S4/S5/S6 draw systems run. It still
/// spawns NO sprite this slice — later GTW-48 slices add the draw systems (S4/S5/S6)
/// behind it. Kept as the top-down-specific home so the future iso swap replaces only
/// this renderer plugin, never shared draw logic.
///
/// The atlas-load system is gated on [`Assets<TextureAtlasLayout>`] existing so the
/// plugin still builds under `MinimalPlugins` (the renamed S1 unit tests add no
/// asset stack): without the sprite/asset plugins that collection is absent and the
/// load is a no-op rather than a param-validation failure. Under `DefaultPlugins`
/// (the app and the AC4 load harness) `SpritePlugin`'s `TextureAtlasPlugin` provides
/// it, so the load runs for real.
///
/// GTW-218 (S4) adds the static terrain draw here: it registers the
/// [`RonAsset<TileRoles>`](gdtf_assets::RonAsset) loader + the
/// [`load_tile_roles`] / [`resolve_tile_roles`] load chain (BOTH gated on an
/// [`AssetServer`] so a `MinimalPlugins` app no-ops rather than panicking on the asset
/// registration), inserts the [`ActiveLevel`] default (level 0), defines the
/// [`PresenterSystems::Draw`] set `.after(SimSystems::Simulate)`, and registers the
/// one-shot [`draw_static_battlefield`] + the [`swap_destroyed_cover`] /
/// [`swap_destroyed_slab`] destruction reactions (GTW-367) in that set, all gated
/// `run_if(resource_exists::<BattleInProgress>)` (the sim's battle-in-progress witness, so
/// the draw runs only DURING a live battle).
pub struct TopDownRendererPlugin;

impl Plugin for TopDownRendererPlugin {
    fn build(&self, app: &mut App) {
        // GTW-348: the terrain-fog material pipeline. Terrain tiles render through a
        // `Material2d` (a unit-rect Mesh2d + MeshMaterial2d<TerrainFogMaterial>) so an
        // EXPLORED cell can render full-brightness GREYSCALE — the `Sprite` pipeline's
        // per-channel multiply tint cannot desaturate. `Material2dPlugin` registers the
        // `Assets<TerrainFogMaterial>` store + (when a `RenderApp` is present) the render
        // pipeline; `DefaultPlugins` registers the built-in materials but NOT a custom one.
        // It is gated on an `AssetServer` (like `register_ron_tables` below): `init_asset`
        // needs the asset machinery, so a `MinimalPlugins` headless app (no asset stack)
        // skips it — and there the draw / fog systems never run anyway (they are gated on
        // `TopDownAtlases`, which only loads with an `AssetServer`), so the absent
        // `Assets<TerrainFogMaterial>` store is never reached (`bevy-traps.md` #1).
        if app.world().get_resource::<AssetServer>().is_some() {
            app.add_plugins(Material2dPlugin::<TerrainFogMaterial>::default());
        }
        app.insert_resource(TopDownRendererActive)
            .init_resource::<ActiveLevel>()
            // GTW-521 — the presenter-owned view mode (DownToActive default = the current
            // GTW-519/520 behaviour). The input crate's ToggleFullView intent flips it; the
            // terrain draw + ganger visibility filter read it through the shared `drawn_band`
            // helper. Present for the whole battle span so the toggle always has a resource.
            .init_resource::<ViewMode>()
            // GTW-358 — the route path-preview read-seam (the find_path route from the selected
            // ganger to the target + its §48 total cost). Present for the whole battle span: the
            // input crate POPULATES it (clearing it when there is no target or it is
            // unreachable); the draw system READS it. Its Default is the empty preview, so a
            // battle with no target draws no route.
            .init_resource::<PathPreview>()
            // GTW-371 — the fire-target highlight read-seam (the hovered fireable-enemy cell +
            // the fire TU cost). Present for the whole battle span: the input crate POPULATES it
            // (clearing it off any non-fireable hover); the draw system READS it. Its Default is
            // the empty highlight, so a battle with no fireable hover draws no fire target.
            .init_resource::<FireTargetHighlight>()
            // The S5 ganger-sprite map (sim Entity -> presenter Entity), present for the
            // whole battle span so the spawn / move / reframe / death / removal systems
            // share one mapping.
            .init_resource::<GangerSprites>()
            .add_systems(
                Startup,
                load_topdown_atlases
                    .run_if(resource_exists::<Assets<bevy::image::TextureAtlasLayout>>),
            );

        // The RON-asset registration (`init_ron_asset` / `init_asset`) PANICS at
        // registration without an `AssetServer` (no `Assets<T>` machinery), so the whole
        // table-load chain is gated on the asset stack being present and lives in
        // `register_ron_tables` (extracted to keep this `build` under the `too_many_lines`
        // lint). Under `DefaultPlugins` it runs for real; under `MinimalPlugins` it is
        // skipped entirely (no draw, no panic).
        register_ron_tables(app);

        // The shared presenter draw band (S5's ganger draw joins this set). Defined
        // ONCE via `configure_sets` (`bevy-traps.md` #5), ordered after the sim's
        // world mutations (`bevy-traps.md` #3) so a draw observes a settled sim state.
        //
        // Both draw systems are gated `run_if(resource_exists::<BattleInProgress>)` (the
        // contract's battle gate) AND on the resources they READ existing: a battle can
        // be `BattleInProgress` while the renderer's `TileRoles` / `TopDownAtlases` are
        // absent (a `MinimalPlugins` headless app with no `AssetServer` never loads
        // them), so without those extra guards the systems would fail param validation
        // when the resource is missing — the exact panic `bevy-traps.md` #1 (and the
        // ticket's "a no-resource state must NOT panic the draw") demands we gate. The
        // draw scans the three sim grids + the two render resources; the swap reaction
        // needs only `TileRoles` (and the always-present `ActiveLevel`).
        app.configure_sets(Update, PresenterSystems::Draw.after(SimSystems::Simulate))
            .add_systems(
                Update,
                draw_static_battlefield
                    .in_set(PresenterSystems::Draw)
                    .run_if(
                        resource_exists::<BattleInProgress>
                            .and_then(resource_exists::<TileRoles>)
                            .and_then(resource_exists::<TopDownAtlases>)
                            .and_then(resource_exists::<OccupancyGrid>)
                            .and_then(resource_exists::<CoverLedger>)
                            .and_then(resource_exists::<SurfaceGrid>),
                    ),
            );

        // GTW-367: the cover + slab destruction-swap reactions (extracted to keep `build`
        // under the `too_many_lines` lint; mirrors `register_fx_flash_systems` / `register_fog_systems`).
        register_destruction_swaps(app);

        // GTW-359 (AC4 / C2) + GTW-373: the vertical-link (stair / ladder) cell draw. It
        // reads the sim's `VerticalLinkGraph` + the presenter's `TileRoles` /
        // `TopDownAtlases` and draws one direction-keyed stair (up 29 / down 28) / ladder
        // (235) tile per authored link endpoint on the active storey (the hard cut),
        // pooled + mutated in place (C5). Gated on
        // `BattleInProgress` (the live-battle witness) AND on every resource it reads:
        // `VerticalLinkGraph` (inserted by the sim's setup_battle, absent in a focused
        // harness that opens BattleInProgress directly), `TileRoles`, and `TopDownAtlases`
        // (a `MinimalPlugins` headless app with no `AssetServer` never loads the latter two)
        // — so a no-resource state simply does not draw rather than panicking param
        // validation (`bevy-traps.md` #1).
        app.add_systems(
            Update,
            draw_vertical_links.in_set(PresenterSystems::Draw).run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<VerticalLinkGraph>)
                    .and_then(resource_exists::<TileRoles>)
                    .and_then(resource_exists::<TopDownAtlases>),
            ),
        );

        // GTW-219 (S5): the ganger-draw change-detection systems join the SAME
        // `PresenterSystems::Draw` band (defined once above, ordered after the sim's
        // mutations). Each is gated `run_if(resource_exists::<BattleInProgress>)` AND on
        // every render resource it reads — `CharacterRoles` (the data table) and
        // `TopDownAtlases` — so a `MinimalPlugins` headless app with no `AssetServer`
        // (those resources absent) simply does not draw rather than failing param
        // validation (`bevy-traps.md` #1; the ticket's "a no-resource state must NOT
        // panic the draw"). `ActiveLevel` + `GangerSprites` are `init_resource`-d on
        // build, so they are always present. `move_ganger_sprites` runs
        // `.after(spawn_ganger_sprites)` so a same-update spawn is already mapped when
        // the move runs (the idempotent-via-the-map move path).
        // GTW-331: `update_ganger_life_state` drains `MessageReader<ShotFired>` to discriminate a
        // shot-kill (deferred to the impact despawn) from a non-shot death (despawned promptly) —
        // see its docs. A `MessageReader` panics param validation without its `Messages<ShotFired>`
        // buffer (`bevy-traps.md` #4), and the sim's `BattleSimPlugin` only registers it in a real
        // battle, so register it idempotently HERE (the `ShotImpactResolved` / `HighlightRequest`
        // precedent): `add_message` creates the buffer if absent and is a no-op if the sim already
        // did — so the ganger batch's gate stays `BattleInProgress + CharacterRoles +
        // TopDownAtlases` (an empty buffer is fine), and the life-state system keeps running for
        // the Downed re-tint + non-shot death despawn even in a focused harness that fires nothing.
        app.add_message::<ShotFired>();
        let gate = resource_exists::<BattleInProgress>
            .and_then(resource_exists::<CharacterRoles>)
            .and_then(resource_exists::<TopDownAtlases>);
        app.add_systems(
            Update,
            (
                spawn_ganger_sprites,
                move_ganger_sprites.after(spawn_ganger_sprites),
                reframe_ganger_sprites,
                update_ganger_life_state,
                apply_active_level_filter,
            )
                .in_set(PresenterSystems::Draw)
                .run_if(gate),
        )
        // GTW-375 (C4): RE-INDEX every mapped ganger sprite to the freshly-reloaded atlas
        // indices when `character_roles.ron` hot-reloads (redrive_character_roles_on_asset_event
        // above marks CharacterRoles changed). The per-field draw systems only re-read the
        // faction base on an Added/Changed SIM event, never on a resource change, so an
        // already-spawned idle ganger needs this dedicated re-index. Gated on `CharacterRoles`
        // existing (so a `MinimalPlugins` app without the table never runs it — `bevy-traps.md`
        // #1) AND `CharacterRoles.is_changed()` so it does no per-frame work; it needs NO
        // battle-witness gate (it is idempotent and its sim/sprite queries are empty pre-battle,
        // per the Discovery). It RE-INDEXES ONLY (it never re-tints), so it never crosses the
        // life/stance/aiming tint writers.
        .add_systems(
            Update,
            reindex_ganger_sprites_on_character_roles_change
                .in_set(PresenterSystems::Draw)
                .run_if(
                    resource_exists::<CharacterRoles>.and_then(resource_changed::<CharacterRoles>),
                ),
        )
        // The removal-detection despawn needs NO render resource (it only despawns
        // mapped sprites + drops map entries), so it is gated on the battle witness
        // alone — it must still run when the table / atlas happen to be absent so a
        // removed ganger never leaves an orphan sprite.
        .add_systems(
            Update,
            despawn_removed_ganger_sprites
                .in_set(PresenterSystems::Draw)
                .run_if(resource_exists::<BattleInProgress>),
        )
        // GTW-359 (AC3 / C4): the general sprite-movement glide. It ticks each ganger
        // sprite's `SpriteTween` (re-targeted by `move_ganger_sprites`) and writes the
        // interpolated `Transform.translation` IN PLACE — so a move is GLIDED, never
        // snapped. Ordered `.after(move_ganger_sprites)` so a same-frame re-target glides
        // this frame. Like `despawn_removed_ganger_sprites` it needs no render resource
        // (only `Res<Time>` + the `(Transform, SpriteTween)` query) and is inert with no
        // tweened sprites, so it is gated on the battle witness alone — an in-flight glide
        // still completes regardless of the table / atlas being present.
        .add_systems(
            Update,
            advance_sprite_tweens
                .in_set(PresenterSystems::Draw)
                .after(move_ganger_sprites)
                .run_if(resource_exists::<BattleInProgress>),
        )
        // GTW-331: the SHOT-KILL death-despawn. It drains the shared GTW-328
        // `ShotImpactResolved` signal `animate_impact` emits at each staggered impact and
        // despawns a ganger whose killing tracer just LANDED (its threaded report struck the
        // ganger + left it Dead) — so the body does not vanish at sim-drain time, before the
        // bolt reaches it. Like `despawn_removed_ganger_sprites` it only despawns mapped sprites
        // + drops map entries (no render resource), so it is gated on the battle witness AND its
        // `Messages<ShotImpactResolved>` buffer (a `MessageReader` panics validation without its
        // buffer — `bevy-traps.md` #4); the buffer is registered unconditionally in
        // `register_fx_flash_systems` (idempotent `add_message`), so the gate is satisfied
        // whenever a battle is live. The `Changed<LifeState>` path (above) defers a shot-kill to
        // this system, so the two never double-despawn (the map entry drops exactly once).
        .add_systems(
            Update,
            despawn_killed_ganger_on_impact
                .in_set(PresenterSystems::Draw)
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<Messages<ShotImpactResolved>>),
                ),
        );

        // GTW-220 (S6): the transient FX-flash readers + the one-shot expiry clock join the
        // SAME `PresenterSystems::Draw` band (extracted to keep `build` under the
        // `too_many_lines` lint).
        register_fx_flash_systems(app);

        // GTW-249: the battle-start frame-on-units + the bounds clamp (extracted for the
        // same `too_many_lines` reason).
        register_camera_framing_systems(app);

        // GTW-251: the message-driven hover-highlight DRAW. The input crate (the CONSUMER's
        // upstream writer) EMITS `HighlightRequest`; this presenter (which DEFINES it, the
        // one-way `input -> presenter -> sim` edge) LISTENS and draws.
        register_highlight_systems(app);

        // GTW-342: the squad fog WRITER. It modulates the already-drawn terrain layer +
        // hard-cuts actor sprites from the sim's `SquadVisibility` (extracted to keep
        // `build` under the `too_many_lines` lint).
        register_fog_systems(app);

        // GTW-358 / GTW-368: the route path-preview DRAW (the per-step route sprites + the C5
        // off-storey link marker + the GTW-368 single target-cell TU-cost label). It reads the
        // presenter-owned `PathPreview` (populated by the input crate) and the sim's
        // `SquadVisibility` (the §53 VISIBLE-vs-EXPLORED dim) and hard-cuts to the active storey
        // (extracted to keep `build` under the `too_many_lines` lint).
        register_path_preview_systems(app);

        // GTW-450: the reachable-range overlay is the DEBUG-only overlay (visual noise in
        // normal play). EVERY overlay-render-only item — the `ReachableCells` read-seam, the
        // `ReachableOverlayEnabled` flag (seeded ONCE here from the env var), and the DRAW
        // system (`run_if` the flag) — compiles ONLY under `#[cfg(debug_assertions)]` (C1).
        // A release build (debug_assertions=false) excludes all of it; the move feedback is
        // then the click-to-target route preview alone (C4). Extracted to keep `build` under
        // the `too_many_lines` lint.
        #[cfg(debug_assertions)]
        register_reachable_overlay_systems(app);

        // GTW-371: the fire-target highlight DRAW (the red under-actor tile + the opaque TU-cost
        // label). It reads the presenter-owned `FireTargetHighlight` (populated by the input
        // crate) and hard-cuts to the active storey (extracted to keep `build` under the
        // `too_many_lines` lint).
        register_fire_target_systems(app);
    }
}

/// Registers the AssetServer-gated RON-table load chains: the S4 [`TileRoles`], the S5
/// [`CharacterRoles`], the S6 [`EffectRoles`], the GTW-306 firing-FX [`FxTuning`], and the
/// GTW-299 edge-pan [`PanTuning`] — each loaded through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader (extracted from `build` to keep it under the
/// `too_many_lines` lint).
///
/// The whole block is gated on an [`AssetServer`] existing: `init_ron_asset` PANICS at
/// registration without the `Assets<T>` machinery, so a `MinimalPlugins` headless app (no asset
/// stack) skips the entire chain — no load, no panic (`bevy-traps.md` #1). Under `DefaultPlugins`
/// (the app + the `AssetServer` harness) every table loads for real.
///
/// Each table follows the SAME load / resolve / (for the two hot-reloadable tuning tables)
/// redrive shape: a `Startup` load that stores the typed handle, an `Update` resolve gated
/// `run_if(handle present AND resource not yet resolved)` so it inserts the resolved resource
/// ONCE, and — for [`FxTuning`] / [`PanTuning`] — an unguarded `Update` redrive that re-derives
/// the resource LIVE on a matching asset `Modified` event so an `.ron` edit hot-reloads without a
/// rebuild (the redrive self-gates on its resources being present, taking them as `Option`s).
fn register_ron_tables(app: &mut App) {
    if app.world().get_resource::<AssetServer>().is_none() {
        return;
    }
    app.init_ron_asset::<TileRoles>()
        .init_ron_asset::<CharacterRoles>()
        // GTW-220 (S6): the FX-flash effect-role table loads the same RON way.
        .init_ron_asset::<EffectRoles>()
        // GTW-306 (TUNING): the hot-reloadable firing-FX tuning table loads the same way.
        .init_ron_asset::<FxTuning>()
        // GTW-299 (TUNING): the hot-reloadable edge-pan tuning table loads the same way.
        .init_ron_asset::<PanTuning>()
        .add_systems(
            Startup,
            (
                load_tile_roles,
                load_character_roles,
                load_effect_roles,
                load_fx_tuning,
                load_pan_tuning,
            ),
        )
        .add_systems(
            Update,
            resolve_tile_roles.run_if(
                resource_exists::<TileRolesHandle>.and_then(not(resource_exists::<TileRoles>)),
            ),
        )
        .add_systems(
            Update,
            resolve_character_roles.run_if(
                resource_exists::<CharacterRolesHandle>
                    .and_then(not(resource_exists::<CharacterRoles>)),
            ),
        )
        .add_systems(
            Update,
            resolve_effect_roles.run_if(
                resource_exists::<EffectRolesHandle>.and_then(not(resource_exists::<EffectRoles>)),
            ),
        )
        // GTW-306 (TUNING): resolve the FX tuning ONCE, then re-derive it LIVE on every matching
        // asset Modified event so an `fx.tuning.ron` edit hot-reloads without a rebuild (mirrors
        // the UI theme's redrive-on-AssetEvent). The resolve is gated like the others (handle
        // present, resource not yet resolved); the redrive runs every frame and self-gates on the
        // resources being present (it Options them).
        .add_systems(
            Update,
            resolve_fx_tuning.run_if(
                resource_exists::<FxTuningHandle>.and_then(not(resource_exists::<FxTuning>)),
            ),
        )
        .add_systems(Update, redrive_fx_tuning_on_asset_event)
        // GTW-375 (PRESENTER): hot-reload EVERY battle tile/sprite sheet on BOTH axes — its
        // role `.ron` AND its sheet `.png`. Three role-`.ron` redrives re-resolve their resident
        // resource LIVE on a re-save (mutating the resource): TileRoles (tile_roles.ron),
        // CharacterRoles (character_roles.ron), and EffectRoles (effect_roles.ron). The
        // sheet-image redrive reacts to a re-save of ANY sheet PNG (terrain, characters, effects,
        // portraits, or any future sheet): it maps the reloaded image id back to its sheet, logs
        // each, and `set_changed()`s TileRoles ONLY for the terrain sheet (the custom
        // TerrainFogMaterial bind group is a snapshot and must be re-prepared via
        // draw_static_battlefield's despawn+respawn) — the other sheets are atlas sprites that the
        // sprite pipeline refreshes on its own.
        //
        // The re-render arm per axis:
        // - tile-role + terrain-image → draw_static_battlefield's `roles.is_changed()` trigger
        //   (despawn+respawn the terrain mesh tiles); stair/ladder link sprites rebuild from
        //   TileRoles every frame (draw_vertical_links) and sample the same terrain handle, so
        //   they pick up both reloads with no dedicated system.
        // - character-role → reindex_ganger_sprites_on_character_roles_change (re-indexes each
        //   mapped ganger sprite, registered in the PresenterSystems::Draw band below);
        //   character-image → the sprite pipeline refreshes the texture on its own.
        // - effect-role → future FX flashes/projectiles read the re-resolved EffectRoles on spawn
        //   (flashes are TRANSIENT, so no existing-flash re-index is needed); effect-image → the
        //   sprite pipeline refreshes on its own.
        //
        // All four redrives run every frame and self-gate on their resources being present (they
        // Option them) so a headless app never panics.
        .add_systems(Update, redrive_tile_roles_on_asset_event)
        .add_systems(Update, redrive_character_roles_on_asset_event)
        .add_systems(Update, redrive_effect_roles_on_asset_event)
        .add_systems(Update, redrive_sheet_images_on_asset_event)
        // GTW-299 (TUNING): resolve the pan tuning ONCE, then re-derive it LIVE on every matching
        // asset Modified event so a `pan.tuning.ron` edit hot-reloads without a rebuild — the
        // exact load/resolve/redrive shape the FX tuning above uses.
        .add_systems(
            Update,
            resolve_pan_tuning.run_if(
                resource_exists::<PanTuningHandle>.and_then(not(resource_exists::<PanTuning>)),
            ),
        )
        .add_systems(Update, redrive_pan_tuning_on_asset_event);
}

/// Registers the GTW-367 terrain destruction-swap reactions: the S4 [`swap_destroyed_cover`]
/// (a [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) swaps the cell's terrain sprite to the
/// `rubble` tile) and the GTW-367 [`swap_destroyed_slab`] (a
/// [`SlabDestroyed`](gdtf_battle_sim::SlabDestroyed) swaps it to the `slab_destroyed` tile) — both
/// in the [`PresenterSystems::Draw`] band, MUTATING the existing tile's material in place (no
/// despawn — the UI mutate-not-respawn rule, C7). Extracted from `build` to keep it under the
/// `too_many_lines` lint (mirrors [`register_fx_flash_systems`] / [`register_fog_systems`]).
///
/// Each reaction is gated `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness)
/// AND `resource_exists::<TileRoles>` (read for the destroyed tile index). The slab reaction's
/// [`MessageReader<SlabDestroyed>`](bevy::ecs::message::MessageReader) panics param validation
/// without its `Messages<SlabDestroyed>` buffer (`bevy-traps.md` #4); the sim's `BattleSimPlugin`
/// registers it in a real battle, so it is registered idempotently HERE (the `ShotFired` /
/// `HighlightRequest` precedent — `add_message` creates the buffer if absent and is a no-op
/// otherwise) and gated on its presence, so a focused harness that opens `BattleInProgress`
/// directly never fails validation.
fn register_destruction_swaps(app: &mut App) {
    app.add_message::<SlabDestroyed>()
        .add_systems(
            Update,
            swap_destroyed_cover
                .in_set(PresenterSystems::Draw)
                .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<TileRoles>)),
        )
        .add_systems(
            Update,
            swap_destroyed_slab.in_set(PresenterSystems::Draw).run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<TileRoles>)
                    .and_then(resource_exists::<Messages<SlabDestroyed>>),
            ),
        );
}

/// Registers the GTW-249 camera-positioning systems plus the GTW-250 pan navigation and the
/// GTW-259 gamepad-cursor edge-pan: the one-shot [`frame_camera_on_units`] (centre the
/// [`WorldCamera`](crate::WorldCamera) on the player gangers at battle start), the every-frame
/// [`pan_camera`] (move the camera under mouse-edge / keyboard / gamepad-right-stick
/// navigation), the every-frame [`pan_camera_on_gamepad_cursor_edge`] (pan when the GTW-259
/// gamepad software cursor reaches a screen edge), and the every-frame [`clamp_camera_to_bounds`]
/// (keep the viewport inside the battlefield extent).
///
/// All are battle-scoped (`bevy-traps.md` #1): gated
/// `run_if(resource_exists::<BattleInProgress>` AND `resource_exists::<PlayerFaction>)` —
/// `PlayerFaction` is the sim's player-gang witness the framing reads, inserted/removed on
/// the same `BattleInProgress` window, so none run (and none panic on a missing `Res`)
/// outside a live battle. The SAME gate keeps the pans and the clamp in the same scheduled
/// band, so the clamp stays the last writer.
///
/// The clamp is ordered `.after` the framing and BOTH pans
/// (`pan_camera.before(clamp_camera_to_bounds)`,
/// `pan_camera_on_gamepad_cursor_edge.before(clamp_camera_to_bounds)`), so it is the LAST
/// writer of the camera position each frame: whatever a pan adds to the translation, the
/// clamp pulls back inside the battlefield bounds — the camera can never be panned off the
/// map (GTW-250 / GTW-259). The pans are view-only: they move the presenter-owned camera
/// `Transform` and emit NO sim message. They run in plain `Update` (camera positioning needs
/// no `PresenterSystems::Draw` membership — it touches no atlas / sprite, only the camera
/// `Transform`).
///
/// The GTW-259 [`GamepadCursorMoved`] message buffer is registered here via
/// [`App::add_message`]: a [`MessageReader<GamepadCursorMoved>`](bevy::ecs::message::MessageReader)
/// panics param validation without its `Messages<GamepadCursorMoved>` buffer
/// (`bevy-traps.md` #4), and `add_message` is IDEMPOTENT — the input crate also registers the
/// same buffer so its `MessageWriter` validates headlessly, and the two coexist (the
/// [`HighlightRequest`] precedent: the presenter DEFINES the message; input WRITES it,
/// input→presenter, no cycle).
///
/// The GTW-299 edge-pan DWELL accumulator ([`PanEdgeDwellState`]) is initialised here with
/// [`App::init_resource`] so it is always present for the (battle-gated) pan systems to read — it
/// is lightweight VIEW state with a [`Default`] (both accumulators zero), so a headless app gets
/// it for free and the dwell gate exercises the real path rather than the `Option`-absent
/// fallback. The pan systems still take it as `Option<ResMut<…>>` so they never panic if it is
/// somehow absent (`bevy-traps.md` #1).
fn register_camera_framing_systems(app: &mut App) {
    let battle_gate =
        resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>);
    app.init_resource::<PanEdgeDwellState>();
    app.add_message::<GamepadCursorMoved>().add_systems(
        Update,
        (
            frame_camera_on_units,
            pan_camera,
            pan_camera_on_gamepad_cursor_edge,
            clamp_camera_to_bounds
                .after(frame_camera_on_units)
                .after(pan_camera)
                .after(pan_camera_on_gamepad_cursor_edge),
        )
            .run_if(battle_gate),
    );
}

/// Registers the GTW-220 (S6) transient FX-flash readers + the one-shot expiry clock into the
/// already-defined [`PresenterSystems::Draw`] band.
///
/// Each reader drains a [`MessageReader`] over one sim FX message
/// ([`Bleeding`](gdtf_battle_sim::Bleeding) / [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) /
/// [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) — the consequence flashes — plus the
/// GTW-290 [`ShotFired`](gdtf_battle_sim::ShotFired) firing FX) the sim already emits, looks
/// up the cell via `Query<&Position>` / the message geometry (read-only, NO sim plumbing
/// added), and `Commands::spawn`s the short-lived effects sprite(s). The GTW-306 firing FX is
/// split across [`spawn_shot_projectiles`] + [`advance_projectiles`] (the traveling directional
/// projectile that LERPs muzzle→impact then despawns, handing off a `PendingImpact`) and
/// [`animate_impact`] (FX-B's 3-frame impact animation at the arrival point); it does NOT
/// duplicate the three consequence flashes. The standalone muzzle flash was REMOVED (GTW-307):
/// it rendered oversized at the shooter's feet and read poorly, so the traveling projectile
/// (departing the muzzle) IS the fire signal.
///
/// Each reader's gate is `resource_exists::<BattleInProgress>` AND every render resource it
/// reads — the [`EffectRoles`] data table + [`TopDownAtlases`] — AND its own `Messages<M>`
/// buffer existing. The render-resource guards make a `MinimalPlugins` headless app with no
/// [`AssetServer`] (those resources absent) simply NOT draw rather than failing param
/// validation (`bevy-traps.md` #1; the ticket's "a no-resource state must NOT panic the
/// draw"). The `Messages<M>` guard is the matching mandatory gate for the [`MessageReader<M>`]
/// param itself: a [`MessageReader<M>`] panics param validation when its `Messages<M>` buffer
/// is absent (the sim's `BattleSimPlugin` registers all three buffers during a real battle,
/// but a focused headless harness may insert `BattleInProgress` + the render tables WITHOUT a
/// given FX buffer), so each reader is independently gated on the one buffer it drains.
///
/// `expire_flashes` is the one-shot despawn clock: it needs only `Res<Time>` + the [`FxFlash`](crate::FxFlash)
/// query (no render resource, no message buffer) and is inert with no flashes (the query is
/// empty), so it is registered unguarded by `BattleInProgress` — a flash spawned during a
/// battle still expires after the battle ends.
fn register_fx_flash_systems(app: &mut App) {
    // GTW-328: register the shared per-shot impact-resolved signal buffer `animate_impact` emits
    // (the combat log drains it). `add_message` is idempotent and creates the `Messages<T>`
    // resource so the `MessageWriter` param is always valid even when `animate_impact` is gated
    // off (`bevy-traps.md` #4 — a MessageWriter panics validation without its buffer); a downstream
    // consumer (the combat-log plugin in `gdtf_app`) also registers it idempotently.
    app.add_message::<ShotImpactResolved>();
    // GTW-507: register the sim's MeleeResolved buffer idempotently so `read_melee_resolved`'s
    // MessageReader param is valid in a presenter-only headless harness (the sim's SimActsPlugin
    // also registers it in a real battle — `add_message` is IDEMPOTENT; the CoverDestroyed
    // precedent, bevy-traps.md #4).
    app.add_message::<MeleeResolved>();
    let render_gate = resource_exists::<BattleInProgress>
        .and_then(resource_exists::<EffectRoles>)
        .and_then(resource_exists::<TopDownAtlases>);
    app.add_systems(
        Update,
        read_bleeding.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<Bleeding>>),
        ),
    )
    .add_systems(
        Update,
        read_armor_broken.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<ArmorBroken>>),
        ),
    )
    .add_systems(
        Update,
        read_cover_destroyed.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<CoverDestroyed>>),
        ),
    )
    // GTW-507: the close-combat STRIKE flash. Drains the sim's MeleeResolved signal (one per
    // connecting melee hit) and spawns a one-frame strike glyph at the struck target cell,
    // data-driven via the EffectRoles `melee_strike` tile (the read_cover_destroyed precedent).
    // Gated on the SAME render resources + the MeleeResolved buffer its MessageReader drains (a
    // MessageReader param panics validation without its buffer — bevy-traps.md #1 / #4).
    .add_systems(
        Update,
        read_melee_resolved.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<MeleeResolved>>),
        ),
    )
    // GTW-306: the firing FX. spawn_shot_projectiles spawns the traveling DIRECTIONAL
    // projectile off the ShotFired buffer (the muzzle flash was removed in GTW-307 — the
    // departing projectile IS the fire signal) under the same render gate; animate_impact
    // (below) drains the SAME buffer independently (a buffered message survives a frame).
    // GTW-327 (slice 2): spawn_shot_projectiles now ALSO classifies each shot's FCT pops
    // (classify_report) + anchor and threads them THROUGH the projectile -> PendingImpact ->
    // animate_impact pipeline, so each shot's numbers appear at its own STAGGERED impact (no
    // separate read_shot_fired_text system — the immediate-spawn that dumped a whole volley's
    // numbers on the drain frame is gone). Both READ the hot-reloadable FxTuning resource, so
    // each adds it to its gate so a pre-resolve frame (tuning not yet loaded) does not fail
    // param validation.
    .add_systems(
        Update,
        spawn_shot_projectiles
            .in_set(PresenterSystems::Draw)
            .run_if(
                render_gate
                    .clone()
                    .and_then(resource_exists::<FxTuning>)
                    .and_then(resource_exists::<Messages<ShotFired>>),
            ),
    )
    // GTW-302 (slice 4): the AUXILIARY-SIGNAL floating-combat-text reader. Drains the SAME
    // Bleeding + ArmorBroken buffers the read_bleeding / read_armor_broken blood/spark flash
    // readers do (a buffered message survives the frame, so both read independently) and spawns
    // a rise/fade Text2d pop per consequence event — "Bleeding" (AMBER) / "Armor Broken" (RED).
    // It spawns Text2d (no effects sprite), so it needs NO render resource — and unlike the
    // per-shot FCT (which now rides the staggered projectile->impact pipeline) the consequence
    // pops are spawned immediately off their own one-shot messages. Gated on BattleInProgress
    // (pops belong to a live battle), BOTH message
    // buffers its two MessageReaders drain (a MessageReader param panics validation without its
    // buffer — bevy-traps.md #1 / #4), AND — GTW-327 — the hot-reloadable FxTuning resource it
    // now READS for the pop lifetime + rise. Reload pops + the numeric "Armor -N" are DEFERRED
    // (no backing sim signal — see the consequence module docs).
    .add_systems(
        Update,
        read_consequence_fct.in_set(PresenterSystems::Draw).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<Messages<Bleeding>>)
                .and_then(resource_exists::<Messages<ArmorBroken>>)
                .and_then(resource_exists::<FxTuning>),
        ),
    )
    // GTW-439 (slice C1): the INJURY floating-combat-text reader. Drains the GTW-438
    // InjuryInflicted message (the buffer the sim's acts plugin registers) and spawns one
    // rise/fade Text2d pop per inflicted injury — its popup_text in a VALENCE BY SEVERITY
    // (the severity_color wound ramp scaled by the rolled tier). The transient flash is the
    // message's ONLY presenter job; the persistent per-ganger injury LIST is driven by the
    // durable InflictedInjuries ledger in the inspect panel, NOT this pop. Spawns Text2d (no
    // effects sprite), so it needs NO render resource. Gated on BattleInProgress (pops belong
    // to a live battle), the InjuryInflicted message buffer its MessageReader drains (a
    // MessageReader param panics validation without its buffer — bevy-traps.md #1 / #4), AND
    // the hot-reloadable FxTuning it reads for the pop lifetime + rise.
    .add_systems(
        Update,
        read_injury_fct.in_set(PresenterSystems::Draw).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<Messages<InjuryInflicted>>)
                .and_then(resource_exists::<FxTuning>),
        ),
    )
    // GTW-306: the 3-frame impact animation (FX-B fills the body). Gated on the same render
    // resources it reads (EffectRoles + TopDownAtlases + BattleInProgress + the
    // hot-reloadable FxTuning) so FX-B edits only impact.rs — never this registration.
    // GTW-328: it ALSO emits the shared per-shot `ShotImpactResolved` signal at each impact
    // (the combat log keys its outcome lines off it). Its buffer is registered just below via
    // `add_message` (idempotent), so the `MessageWriter` param is always valid (`bevy-traps.md`
    // #4) — no extra run gate is needed for the writer (a writer needs only the buffer, which
    // the registration guarantees).
    .add_systems(
        Update,
        animate_impact
            .in_set(PresenterSystems::Draw)
            .run_if(render_gate.and_then(resource_exists::<FxTuning>)),
    )
    // GTW-306: advance every traveling projectile + hand off its impact. Needs only Time + the
    // ShotProjectile query (no render resource / message buffer), inert with none — registered
    // unguarded like expire_flashes so an in-flight projectile completes after a battle ends.
    .add_systems(Update, advance_projectiles.in_set(PresenterSystems::Draw))
    // GTW-302 (slice 2): rise + fade + despawn every live floating-combat-text pop. Like
    // expire_flashes / advance_projectiles it needs only Time + its own (FloatingCombatText)
    // query — no render resource, no message buffer — and is inert with no pops, so it is
    // registered unguarded by BattleInProgress: a pop spawned during a battle still completes
    // its rise/fade after the battle ends. The reader slices (3-4) SPAWN the pops; this is the
    // generic animator the primitive owns.
    .add_systems(Update, animate_floating_text.in_set(PresenterSystems::Draw))
    .add_systems(Update, expire_flashes.in_set(PresenterSystems::Draw));
}

/// Registers the GTW-251 message-driven hover-highlight draw into the already-defined
/// [`PresenterSystems::Draw`] band.
///
/// The presenter DEFINES the [`HighlightRequest`] message (the consumer owns its input
/// API, mirroring how the sim defines the `*Requested` messages input writes) and
/// registers its buffer here via [`App::add_message`] (Bevy 0.18 — buffered events are
/// messages, `bevy-traps.md` #4). The buffer registration is UNCONDITIONAL (not behind
/// the asset gate): a [`MessageReader<HighlightRequest>`](bevy::ecs::message::MessageReader)
/// panics param validation without its `Messages<HighlightRequest>` buffer (`bevy-traps.md`
/// #4), and `add_message` is IDEMPOTENT — the input crate also registers the same buffer
/// so its `MessageWriter` validates headlessly, and the two coexist (the `*Requested`
/// precedent).
///
/// [`draw_highlight_on_request`] joins the SAME `PresenterSystems::Draw` band (defined
/// once, ordered `.after(SimSystems::Simulate)`), gated `run_if(resource_exists::<BattleInProgress>)`
/// — the sim's live-battle witness, the same gate the other draw systems use, so the
/// highlight is inert pre-battle (`bevy-traps.md` #1). It needs NO render resource (it
/// draws a solid-tint sprite, not an atlas tile) and its `Messages<HighlightRequest>`
/// buffer is guaranteed present by the `add_message` above, so the battle gate alone is
/// sufficient. The MIGRATED highlight sprite (the `HoverHighlight` marker + its lazy spawn)
/// now lives in `highlight.rs`; its lifecycle matches the old input-side one (lazily
/// spawned, despawned with the battle world).
fn register_highlight_systems(app: &mut App) {
    app.add_message::<HighlightRequest>().add_systems(
        Update,
        draw_highlight_on_request
            .in_set(PresenterSystems::Draw)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

/// Registers the GTW-342 squad fog WRITER into the already-defined
/// [`PresenterSystems::Draw`] band.
///
/// [`present_fog`] is the VIEW arm of the squad fog (`docs/combat/visibility.md`): the sim
/// owns the three [`SquadVisibility`](gdtf_battle_sim::SquadVisibility) states and the
/// [`recompute_visibility`](gdtf_battle_sim::recompute_visibility) writer (GTW-341); this
/// presenter READS them and modulates the already-drawn layer in place (the rendered layer
/// IS the fog mask — it never repaints from a snapshot).
///
/// # Ordering (the CRITICAL clause, `bevy-traps.md` #3)
///
/// It is ordered strictly `.after(draw_static_battlefield)`, `.after(swap_destroyed_cover)`, and
/// `.after(swap_destroyed_slab)` (GTW-367) so it always colours LIVE, freshly-spawned /
/// just-swapped [`TerrainSprite`](crate::TerrainSprite) entities — including after an
/// [`ActiveLevel`] cycle, which despawns + respawns the terrain on the SAME update (so the fog
/// re-applies to the newly drawn storey, not to stale entities). It
/// is ALSO ordered after the ganger storey-filter writers
/// ([`spawn_ganger_sprites`] / [`move_ganger_sprites`] / [`apply_active_level_filter`]) so it is
/// the SINGLE FINAL writer of each actor sprite's [`Visibility`]: it composes the slice's storey
/// fact AND the fog fact rather than crossing the slice's writer (`docs/combat/visibility.md`
/// §"Composition with the view slice").
///
/// # Gating (`bevy-traps.md` #1)
///
/// `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness) AND
/// `resource_exists::<SquadVisibility>` (the fog sets — inserted by the sim's `setup_battle`,
/// absent in a focused harness that opens `BattleInProgress` directly; without this guard the
/// `Res<SquadVisibility>` param would panic validation) AND
/// `resource_exists::<Assets<TerrainFogMaterial>>` (GTW-348 — the terrain arm drives each tile's
/// material `saturation` via this store; created by `Material2dPlugin` only when an `AssetServer`
/// is present, so a `MinimalPlugins` app without it never runs the writer) AND
/// `resource_exists::<CombatTuning>` (the `Load`-state combat tuning — the "a real battle's
/// balance data is configured" witness). `GangerSprites` + `ActiveLevel` are `init_resource`-d
/// on build, so they are always present.
///
/// GTW-348 NOTE on the `CombatTuning` gate: the EXPLORED treatment no longer reads `explored_dim`
/// (EXPLORED is full-brightness greyscale, not a brightness dim) and `present_fog` no longer takes
/// a `CombatTuning` param — so the gate is no longer a "the param needs this resource" guard. It
/// is KEPT as the battle-configured witness: in the real app `CombatTuning` is always present
/// during a battle (a `Load`-state resource), so fog runs exactly as before; but a focused
/// presenter harness that drives a bare ganger spawn WITHOUT inserting `CombatTuning` (e.g. the
/// storey-filter `ganger_draw` tests) keeps fog INERT, so this change does not silently flip the
/// actor-fog hard-cut on in those harnesses — GTW-348 is a TERRAIN-only change, so the actor arm's
/// observable behavior is held identical to pre-GTW-348 in every harness.
fn register_fog_systems(app: &mut App) {
    app.add_systems(
        Update,
        present_fog
            .in_set(PresenterSystems::Draw)
            .after(draw_static_battlefield)
            .after(swap_destroyed_cover)
            .after(swap_destroyed_slab)
            .after(spawn_ganger_sprites)
            .after(move_ganger_sprites)
            .after(apply_active_level_filter)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<SquadVisibility>)
                    .and_then(resource_exists::<Assets<TerrainFogMaterial>>)
                    .and_then(resource_exists::<CombatTuning>),
            ),
    );
}

/// Registers the GTW-357 reachable-range overlay DRAW systems into the already-defined
/// [`PresenterSystems::Draw`] band.
///
/// Registers the GTW-358 / GTW-368 route path-preview DRAW system into the already-defined
/// [`PresenterSystems::Draw`] band.
///
/// The PRESENTER owns the [`PathPreview`] read-seam (`init_resource`-d on build above) plus
/// this draw system; the INPUT crate POPULATES the resource by calling
/// [`find_path`](gdtf_battle_sim::find_path) for the selected ganger → the target — the
/// [`HighlightRequest`] precedent, where the presenter DEFINES the type and input WRITES it,
/// keeping the `input → presenter → sim` direction (never a cycle).
///
/// [`draw_path_preview`] draws each route step on the active storey with a pooled,
/// mutated-in-place [`Sprite`] (never despawn-respawned), §53-dimmed on a squad-EXPLORED
/// (remembered) step, plus a MINIMAL marker at the cell where the route leaves the active
/// storey (the C5 off-storey-continuation placeholder; the full cross-storey indicator is the
/// GTW-359 soft dep) AND the GTW-368 SINGLE TU-cost [`Text2d`] label at the previewed TARGET
/// cell (the route's last cell), mutated in place + hidden when the preview is empty.
///
/// Gated `run_if(resource_exists::<BattleInProgress>)` — the sim's live-battle witness, the
/// same gate the highlight draw uses (the inert-pre-battle requirement, `bevy-traps.md` #1) —
/// AND `resource_exists::<SquadVisibility>` (the §53 VISIBLE-vs-EXPLORED read: the fog sets are
/// inserted by the sim's `setup_battle`, absent in a focused harness that opens
/// `BattleInProgress` directly; without this guard the `Res<SquadVisibility>` param would panic
/// validation — the [`present_fog`] precedent). It needs NO render resource (a solid-tint sprite
/// and a `Text2d`, not an atlas tile) and the always-present `init_resource`-d [`PathPreview`]
/// and [`ActiveLevel`]. It is ordered `.after(present_fog)` so the route composites OVER the
/// fogged battlefield — the route preview is the topmost within-storey band
/// ([`Layer::PathPreview`](crate::Layer)).
fn register_path_preview_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_path_preview
            .in_set(PresenterSystems::Draw)
            .after(present_fog)
            .run_if(
                resource_exists::<BattleInProgress>.and_then(resource_exists::<SquadVisibility>),
            ),
    );
}

/// Registers the GTW-371 fire-target highlight DRAW system into the already-defined
/// [`PresenterSystems::Draw`] band.
///
/// The PRESENTER owns the [`FireTargetHighlight`] read-seam (`init_resource`-d on build above)
/// plus this draw system; the INPUT crate POPULATES the resource by deciding the fireable-enemy
/// verdict (mirroring `decide_left_click`'s FIRE rung) + computing the
/// [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) — the [`HighlightRequest`] precedent, where the
/// presenter DEFINES the type and input WRITES it, keeping the `input → presenter → sim`
/// direction (never a cycle).
///
/// [`draw_fire_target`] draws the SINGLE red tile UNDER the hovered enemy
/// ([`Layer::FireTarget`](crate::Layer), z below the actor band) with a pooled,
/// mutated-in-place [`Sprite`] (never despawn-respawned) plus the SINGLE OPAQUE TU-cost
/// [`Text2d`] label above the cell, hard-cut to the active storey and hidden off a fireable
/// hover.
///
/// Gated `run_if(resource_exists::<BattleInProgress>)` — the sim's live-battle witness, the same
/// gate the highlight / path-preview draws use (the inert-pre-battle requirement,
/// `bevy-traps.md` #1). It needs NO render resource (a solid-tint sprite + a `Text2d`, not an
/// atlas tile) and NO `SquadVisibility` (the input populate applies the fog gate before writing
/// the seam — the draw only reads the resolved highlight + the always-present `init_resource`-d
/// [`FireTargetHighlight`] / [`ActiveLevel`]). It is ordered `.after(present_fog)` so the red tile
/// composites OVER the fogged battlefield (and, being at [`Layer::FireTarget`](crate::Layer),
/// UNDER the enemy sprite).
fn register_fire_target_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_fire_target
            .in_set(PresenterSystems::Draw)
            .after(present_fog)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

/// Registers the GTW-387 / GTW-450 reachable-range DEBUG overlay: the [`ReachableCells`]
/// read-seam, the [`ReachableOverlayEnabled`] flag (seeded from the env var), and the DRAW
/// system into the already-defined [`PresenterSystems::Draw`] band.
///
/// DEBUG-ONLY (GTW-450 C1): this whole fn — and every item it names — compiles only under
/// `#[cfg(debug_assertions)]`. A release build excludes it, so the overlay never renders
/// in shipping play (it washes the FOV green — visual noise the user ruled out).
///
/// The PRESENTER owns the [`ReachableCells`] read-seam (`init_resource`-d here) plus this
/// draw system; the INPUT crate POPULATES the resource by calling
/// [`reachable_within`](gdtf_battle_sim::reachable_within) for the selected ganger — the
/// [`PathPreview`](crate::PathPreview) precedent, where the presenter DEFINES the type
/// and input WRITES it, keeping the `input → presenter → sim` direction.
///
/// [`draw_reachable_overlay`] draws each reachable cell on the active storey with a
/// pooled, mutated-in-place [`Sprite`] (never despawn-respawned), hard-cut to the active
/// storey. The overlay follows `PageUp` with NO extra wiring: it reads `Res<ActiveLevel>`
/// live every frame.
///
/// The [`ReachableOverlayEnabled`] flag is inserted ONCE here from
/// [`ReachableOverlayEnabled::from_env`] (reading [`REACHABLE_OVERLAY_ENV`] —
/// `GDTF_DEBUG_REACHABLE_OVERLAY`); UNSET → `false` → the draw system's `run_if` is false
/// → NO overlay renders by default (GTW-450 C3 / C4).
///
/// Gated `run_if(resource_exists::<BattleInProgress>)` — the sim's live-battle witness —
/// AND `resource_exists::<SquadVisibility>` so the resource is present (inserted by the
/// sim's `setup_battle`) AND the [`ReachableOverlayEnabled`] flag VALUE (the C3 runtime
/// opt-in; the flag is always present here, so the gate reads its value, not its
/// existence). It needs NO render resource (solid-tint sprites, not atlas tiles) and the
/// always-present `init_resource`-d [`ReachableCells`] and [`ActiveLevel`]. Ordered
/// `.after(present_fog)` so the range tint composites OVER the fogged battlefield,
/// consistent with the path-preview placement.
#[cfg(debug_assertions)]
fn register_reachable_overlay_systems(app: &mut App) {
    // GTW-450 C3 — read the env var ONCE at startup (NOT per-frame) into the flag resource.
    app.insert_resource(ReachableOverlayEnabled::from_env())
        // GTW-387 — the reachable-range overlay read-seam (the cells the selected ganger can
        // reach within its remaining TU). Its Default is the empty set; the input crate
        // POPULATES it (clearing it when no ganger is selected).
        .init_resource::<ReachableCells>()
        .add_systems(
            Update,
            draw_reachable_overlay
                .in_set(PresenterSystems::Draw)
                .after(present_fog)
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<SquadVisibility>)
                        // GTW-450 C3 — the runtime opt-in: render only when the flag is true.
                        .and_then(reachable_overlay_enabled),
                ),
        );
}

/// Run-condition: whether the reachable-range DEBUG overlay is enabled this process
/// (GTW-450 C3) — reads the [`ReachableOverlayEnabled`] flag VALUE. DEBUG-only.
///
/// `Option<Res<…>>` (fail-closed if absent: the focused presenter `build` always inserts
/// it, but a harness might not) so the draw/populate systems stay inert unless the flag is
/// present AND `true`.
#[cfg(debug_assertions)]
fn reachable_overlay_enabled(flag: Option<Res<ReachableOverlayEnabled>>) -> bool {
    flag.is_some_and(|flag| **flag)
}

/// The isometric renderer plugin — a no-op stub for the whole of GTW-48.
///
/// Selected by [`BattlePresenterMode::Iso`]. It stays empty for the whole epic; the
/// real iso renderer is GTW-49 / GTW-10.
pub struct IsoRendererPlugin;

impl Plugin for IsoRendererPlugin {
    fn build(&self, _app: &mut App) {}
}
