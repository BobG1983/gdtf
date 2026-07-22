//! The top-down renderer plugin shell: the active marker, the resource/asset setup,
//! the ONE shared Draw-band definition, the hot-RON chains, and one call per
//! per-concern registrar module.

use bevy::{prelude::*, sprite_render::Material2dPlugin};
use gdtf_battle_sim::occupancy_sync::SimSystems;

use crate::{
    ActiveLevel, FireTargetHighlight, GangerSprites, IsolateView, PathPreview, PresenterSystems,
    TerrainFogMaterial, ViewMode,
    actors::{
        fx::{register_effect_roles_hot_ron, register_fx_tuning_hot_ron},
        ganger::register_character_roles_hot_ron,
    },
    load_topdown_atlases,
    render::{topdown::register_sheet_image_redrive, world_camera::register_pan_tuning_hot_ron},
};

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
/// [`load_topdown_atlases`] in [`Startup`] so the role-keyed
/// [`TopDownAtlases`](crate::TopDownAtlases) resource is built ONCE and present before
/// the S4/S5/S6 draw systems run. It still spawns NO sprite this slice — later GTW-48
/// slices add the draw systems (S4/S5/S6) behind it. Kept as the top-down-specific home
/// so the future iso swap replaces only this renderer plugin, never shared draw logic.
///
/// The atlas-load system is gated on [`Assets<TextureAtlasLayout>`] existing so the
/// plugin still builds under `MinimalPlugins` (the renamed S1 unit tests add no
/// asset stack): without the sprite/asset plugins that collection is absent and the
/// load is a no-op rather than a param-validation failure. Under `DefaultPlugins`
/// (the app and the AC4 load harness) `SpritePlugin`'s `TextureAtlasPlugin` provides
/// it, so the load runs for real.
///
/// The draw-system registrations themselves live in this module's sibling per-concern
/// registrar modules (`terrain` / `gangers` / `fx` / `camera` / `fog` / `overlays`);
/// `build` defines the shared [`PresenterSystems::Draw`] band and its three chained
/// stages ([`PresenterSystems::Scene`] → [`PresenterSystems::Compose`] →
/// [`PresenterSystems::Overlay`], GTW-623) once and calls one registrar per concern.
pub struct TopDownRendererPlugin;

impl Plugin for TopDownRendererPlugin {
    fn build(&self, app: &mut App) {
        // GTW-348: the terrain-fog material pipeline. Terrain tiles render through a
        // `Material2d` (a unit-rect Mesh2d + MeshMaterial2d<TerrainFogMaterial>) so an
        // EXPLORED cell can render full-brightness GREYSCALE — the `Sprite` pipeline's
        // per-channel multiply tint cannot desaturate. `Material2dPlugin` registers the
        // `Assets<TerrainFogMaterial>` store + (when a `RenderApp` is present) the render
        // pipeline; `DefaultPlugins` registers the built-in materials but NOT a custom one.
        // It is gated on an `AssetServer` (like the hot-RON chains below): `init_asset`
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
            // GTW-594 — the orthogonal Isolate toggle (Off default = NO battlescape visual
            // change day one; C3). The storey-treatment classifier composes it with the
            // ViewMode; while On it WINS over the two-state mode (the C3 precedence).
            .init_resource::<IsolateView>()
            // GTW-358 — the route path-preview read-side resource (the find_path route from the selected
            // ganger to the target + its §48 total cost). Present for the whole battle span: the
            // input crate POPULATES it (clearing it when there is no target or it is
            // unreachable); the draw system READS it. Its Default is the empty preview, so a
            // battle with no target draws no route.
            .init_resource::<PathPreview>()
            // GTW-371 — the fire-target highlight read-side resource (the hovered fireable-enemy cell +
            // the fire TU cost). Present for the whole battle span: the input crate POPULATES it
            // (clearing it off any non-fireable hover); the draw system READS it. Its Default is
            // the empty highlight, so a battle with no fireable hover draws no fire target.
            .init_resource::<FireTargetHighlight>()
            // The S5 ganger-sprite map (sim Entity -> presenter Entity), present for the
            // whole battle span so the spawn / move / appearance / death / removal systems
            // share one mapping.
            .init_resource::<GangerSprites>()
            .add_systems(
                Startup,
                load_topdown_atlases
                    .run_if(resource_exists::<Assets<bevy::image::TextureAtlasLayout>>),
            );

        // GTW-564: each hot-RON table registers through the generic registration helper at its OWNING
        // module — one ext call per chain, gathered by `register_hot_ron_chains` below
        // (which replaced the old `register_ron_tables` wall of per-site
        // load/resolve/redrive registrations; extracted to keep `build` under the
        // `too_many_lines` lint).
        register_hot_ron_chains(app);

        // The shared presenter draw band. Defined ONCE via `configure_sets`
        // (`bevy-traps.md` #5), ordered after the sim's world mutations
        // (`bevy-traps.md` #3) so a draw observes a settled sim state.
        // GTW-727: the draw band is re-anchored `.after(SimSystems::Record)` (which is
        // itself `.after(Simulate)`), so a draw observes not only a settled sim state but a
        // settled RECORD of how the sim reached it — the playback cursor in `Replay` can
        // therefore consume this tick's entries in the same update they were written.
        app.configure_sets(Update, PresenterSystems::Draw.after(SimSystems::Record));
        // GTW-623: the three chained DRAW STAGES inside that band — Scene (the drawn
        // world: terrain + swaps + gangers) → Compose (fog: the final material +
        // visibility writer) → Overlay (highlight / path / fire / field / reachable +
        // FX + the FCT palette). Configured ONCE here (the ConsequenceFctSystems
        // Reset→Read exemplar); every draw system gets its ordering from STAGE
        // MEMBERSHIP, so the old cross-stage pairwise `.after` wall (fog's seven
        // edges, the overlays' four `.after(present_fog)`) is GONE — a NEW scene
        // writer (e.g. a doors state-swap) joins `Scene` and fog + overlays order
        // after it with zero new edges. Pairwise `.after` stays only for true
        // data-flow INSIDE a stage (ganger move-after-spawn, tween-after-move).
        app.configure_sets(
            Update,
            (
                PresenterSystems::Replay,
                PresenterSystems::Scene,
                PresenterSystems::Compose,
                PresenterSystems::Overlay,
            )
                .chain()
                .in_set(PresenterSystems::Draw),
        );

        // GTW-727: the playback cursor — its two process-lifetime resources, every
        // `Played<M>` buffer, the hot-reloadable dwell table, and the two `Replay`-stage
        // systems. Registered ONLY here, so an app without this renderer has no cursor at
        // all and the input gate can never close there (clause (d), structurally).
        crate::playback::register_playback(app);

        // GTW-218 (S4): the static terrain draw.
        super::terrain::register_terrain_draw(app);

        // GTW-367: the cover + slab destruction-swap reactions (extracted to keep `build`
        // under the `too_many_lines` lint; mirrors `register_fx_flash_systems` / `register_fog_systems`).
        super::terrain::register_destruction_swaps(app);

        // GTW-359 (AC4 / C2) + GTW-373: the vertical-link (stair / ladder) cell draw.
        super::terrain::register_vertical_links(app);

        // GTW-219 (S5): the ganger-sprite draw-band systems.
        super::gangers::register_ganger_draw(app);

        // GTW-220 (S6): the transient FX-flash readers (incl. the GTW-546 grenade BLAST
        // reader) + the firing FX + the one-shot expiry clock join the Overlay stage —
        // the six flash-family readers via the GTW-623 `FxReaderAppExt` registrar.
        super::fx::register_fx_flash_systems(app);

        // GTW-249: the battle-start frame-on-units + the bounds clamp (extracted for the
        // same `too_many_lines` reason).
        super::camera::register_camera_framing_systems(app);

        // GTW-251: the message-driven hover-highlight DRAW. The input crate (the CONSUMER's
        // upstream writer) EMITS `HighlightRequest`; this presenter (which DEFINES it, the
        // one-way `input -> presenter -> sim` edge) LISTENS and draws.
        super::overlays::register_highlight_systems(app);

        // GTW-342: the squad fog WRITER. It modulates the already-drawn terrain layer +
        // hard-cuts actor sprites from the sim's `SquadVisibility` (extracted to keep
        // `build` under the `too_many_lines` lint).
        super::fog::register_fog_systems(app);

        // GTW-358 / GTW-368: the route path-preview DRAW (the per-step route sprites + the C5
        // off-storey link marker + the GTW-368 single target-cell TU-cost label). It reads the
        // presenter-owned `PathPreview` (populated by the input crate) and the sim's
        // `SquadVisibility` (the §53 VISIBLE-vs-EXPLORED dim) and hard-cuts to the active storey
        // (extracted to keep `build` under the `too_many_lines` lint).
        super::overlays::register_path_preview_systems(app);

        // GTW-545: the area-damage-field overlay DRAW (the persistent per-cell hazard wash). It
        // reads the AUTHORITATIVE sim `FieldRegistry` (seeded by `setup_battle`) one-way and draws
        // one translucent tile per fielded cell on the active storey, so a seeded field (a toxic
        // pool) is VISIBLE. A SHIPPING view (the playability rule), NOT debug-gated. Extracted to
        // keep `build` under the `too_many_lines` lint.
        super::overlays::register_field_overlay_systems(app);

        // GTW-572: the CONSEQUENCE-FCT PALETTE — the shared per-frame stack counter + reset,
        // then one registrar line per consequence family (bleeding / armor-broken / injury /
        // suppression / DOT / field / on-death). Replaces the six hand-rolled per-family
        // reader registrations and the two presenter-side idempotent add_message calls
        // (FieldTicked / OnDeathOccurred — C4: the sim registers those buffers in live play).
        super::fx::register_consequence_fct_families(app);

        // GTW-620: the combat-log FORWARDERS (sim fact -> resolved CombatLogEvent), moved
        // down from gdtf_app beside the log_event vocabulary they feed — one registrar line
        // per source over the same sim facts the FCT drains read. gdtf_app's appender (the
        // bevy_ui half) orders itself .after(CombatLogSystems::Forward) cross-crate.
        super::combat_log::register_combat_log_forwarders(app);

        // GTW-450: the reachable-range overlay is the DEBUG-only overlay (visual noise in
        // normal play). EVERY overlay-render-only item — the `ReachableCells` read-side resource, the
        // `ReachableOverlayEnabled` flag (seeded ONCE here from the env var), and the DRAW
        // system (`run_if` the flag) — compiles ONLY under `#[cfg(debug_assertions)]` (C1).
        // A release build (debug_assertions=false) excludes all of it; the move feedback is
        // then the click-to-target route preview alone (C4). Extracted to keep `build` under
        // the `too_many_lines` lint.
        #[cfg(debug_assertions)]
        super::overlays::register_reachable_overlay_systems(app);

        // GTW-371: the fire-target highlight DRAW (the red under-actor tile + the opaque TU-cost
        // label). It reads the presenter-owned `FireTargetHighlight` (populated by the input
        // crate) and hard-cuts to the active storey (extracted to keep `build` under the
        // `too_many_lines` lint).
        super::overlays::register_fire_target_systems(app);

        // GTW-596: the cross-level tactical badges — the CrossLevelSignals DERIVE (Compose,
        // reading the SAME fog resources present_fog / the ganger-visibility resolver use) + the
        // badge DRAW (Overlay, change-driven off the resource). Extracted to keep `build`
        // under the `too_many_lines` lint.
        super::overlays::register_cross_level_signals_systems(app);
    }
}

/// Gathers the per-module hot-RON chain registrations (GTW-564): one generic-registration
/// ext call per table —
/// [`CharacterRoles`](crate::CharacterRoles), [`EffectRoles`](crate::EffectRoles),
/// [`FxTuning`](crate::FxTuning), [`PanTuning`](crate::PanTuning) — plus the one
/// NON-RON reaction (the sheet-image redrive owned by `render/topdown/redrive.rs`).
/// (The terrain `TileRoles` chain RETIRED with its table — GTW-665: terrain graphics
/// now resolve through the GTW-663 sprite-def family the HOST's Load pass registers.)
/// Each ext call wires the chain's kick-off / gated resolve / live redrive and
/// SELF-gates on the [`AssetServer`](bevy::asset::AssetServer), so a `MinimalPlugins`
/// headless app skips every chain (no load, no panic — `bevy-traps.md` #1). This
/// replaced the `register_ron_tables` wall (extracted from `build` to keep it under
/// the `too_many_lines` lint).
fn register_hot_ron_chains(app: &mut App) {
    register_character_roles_hot_ron(app);
    // GTW-220 (S6): the FX-flash effect-role table.
    register_effect_roles_hot_ron(app);
    // GTW-306 (TUNING): the hot-reloadable firing-FX tuning table.
    register_fx_tuning_hot_ron(app);
    // GTW-299 (TUNING): the hot-reloadable edge-pan tuning table.
    register_pan_tuning_hot_ron(app);
    // GTW-375: the sheet-IMAGE (`.png`) redrive — the one non-RON hot-reload
    // reaction, registered by its owning module (render/topdown/redrive.rs).
    register_sheet_image_redrive(app);
}
