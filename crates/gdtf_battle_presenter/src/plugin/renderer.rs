//! The presenter mode selector and the renderer plugins, with the top-down renderer's
//! full system wiring.

use bevy::{ecs::message::Messages, prelude::*, sprite_render::Material2dPlugin};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_sim::{
    ArmorBroken, BattleInProgress, Bleeding, CombatTuning, CoverDestroyed, CoverLedger,
    OccupancyGrid, PlayerFaction, ShotFired, SquadVisibility, SurfaceGrid,
    occupancy_sync::SimSystems,
};

use crate::{
    ActiveLevel, CharacterRoles, CharacterRolesHandle, EffectRoles, EffectRolesHandle, FxTuning,
    FxTuningHandle, GamepadCursorMoved, GangerSprites, HighlightRequest, PanEdgeDwellState,
    PanTuning, PanTuningHandle, PresenterSystems, ShotImpactResolved, TerrainFogMaterial,
    TileRoles, TileRolesHandle, TopDownAtlases, advance_projectiles, animate_floating_text,
    animate_impact, apply_active_level_filter, clamp_camera_to_bounds,
    despawn_killed_ganger_on_impact, despawn_removed_ganger_sprites, draw_highlight_on_request,
    draw_static_battlefield, expire_flashes, frame_camera_on_units, load_character_roles,
    load_effect_roles, load_fx_tuning, load_pan_tuning, load_tile_roles, load_topdown_atlases,
    move_ganger_sprites, pan_camera, pan_camera_on_gamepad_cursor_edge, present_fog,
    read_armor_broken, read_bleeding, read_consequence_fct, read_cover_destroyed,
    redrive_fx_tuning_on_asset_event, redrive_pan_tuning_on_asset_event, reframe_ganger_sprites,
    resolve_character_roles, resolve_effect_roles, resolve_fx_tuning, resolve_pan_tuning,
    resolve_tile_roles, spawn_ganger_sprites, spawn_shot_projectiles, swap_destroyed_cover,
    update_ganger_life_state,
};

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
/// one-shot [`draw_static_battlefield`] + the [`swap_destroyed_cover`] reaction in that
/// set, both gated `run_if(resource_exists::<BattleInProgress>)` (the sim's
/// battle-in-progress witness, so the draw runs only DURING a live battle).
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
            )
            .add_systems(
                Update,
                swap_destroyed_cover.in_set(PresenterSystems::Draw).run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<TileRoles>),
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
        // asset Modified event so an `fx_tuning.ron` edit hot-reloads without a rebuild (mirrors
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
        // GTW-299 (TUNING): resolve the pan tuning ONCE, then re-derive it LIVE on every matching
        // asset Modified event so a `pan_tuning.ron` edit hot-reloads without a rebuild — the
        // exact load/resolve/redrive shape the FX tuning above uses.
        .add_systems(
            Update,
            resolve_pan_tuning.run_if(
                resource_exists::<PanTuningHandle>.and_then(not(resource_exists::<PanTuning>)),
            ),
        )
        .add_systems(Update, redrive_pan_tuning_on_asset_event);
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
/// It is ordered strictly `.after(draw_static_battlefield)` and `.after(swap_destroyed_cover)`
/// so it always colours LIVE, freshly-spawned / just-swapped [`TerrainSprite`](crate::TerrainSprite)
/// entities — including after an [`ActiveLevel`] cycle, which despawns + respawns the terrain on
/// the SAME update (so the fog re-applies to the newly drawn storey, not to stale entities). It
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

/// The isometric renderer plugin — a no-op stub for the whole of GTW-48.
///
/// Selected by [`BattlePresenterMode::Iso`]. It stays empty for the whole epic; the
/// real iso renderer is GTW-49 / GTW-10.
pub struct IsoRendererPlugin;

impl Plugin for IsoRendererPlugin {
    fn build(&self, _app: &mut App) {}
}
