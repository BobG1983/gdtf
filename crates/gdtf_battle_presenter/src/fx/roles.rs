//! The DATA-DRIVEN FX-role table and its RON load/resolve chain.

use bevy::{asset::AssetEvent, math::Vec3, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::DamageType;
use serde::Deserialize;

use crate::TileIndex;

/// How many compass directions the directional-shot strip carries — the 8-way
/// rose `assets/tiles/alt_tileset_effects.png` authors per damage-type row.
///
/// A `const`, NOT a domain newtype (the framework-plumbing carve-out,
/// `.claude/rules/no-bare-types.md` clause 4): a fixed array LENGTH, the same
/// reasoning the `CELL_PX`-class consts use. It pins both the `.ron` strip length
/// and the [`DamageTypeFx::directions`] array arity, kept in lock-step with
/// [`COMPASS_DIRECTIONS`].
pub const DIRECTION_COUNT: usize = 8;

/// How many frames the per-damage-type impact ANIMATION holds — the 3-tile
/// expanding-shockwave sequence (solid burst -> open ring -> breaking ring) the
/// sheet authors per row, played by FX-B's `animate_impact`.
///
/// A `const`, NOT a domain newtype (the array-arity carve-out): it pins the
/// [`DamageTypeFx::impact`] array length AND the `.ron` impact-strip length.
pub const IMPACT_FRAME_COUNT: usize = 3;

/// The 8 compass directions, in the SAME column order the sheet authors a
/// damage-type row's directional-shot strip (read off `alt_tileset_effects.png`
/// cols 0..8): the unit XY heading of the projectile tile at each strip index.
///
/// Column `i` of a row's directional strip is the projectile drawn for heading
/// [`COMPASS_DIRECTIONS`]`[i]`, so [`nearest_direction_index`] maps a shot's
/// trajectory to its tile by picking the nearest of these. Headings are in
/// WORLD-SCREEN XY (`+x` right, `+y` UP), the same frame
/// [`sim_pos_to_world`](crate::sim_pos_to_world) projects into — a sim
/// trajectory's `+y` ("north" in sim cells) maps to screen `+y` so the rose reads
/// upright. The authored column order is:
///
/// ```text
/// 0 E · 1 SE · 2 S · 3 SW · 4 N · 5 NW · 6 W · 7 NE
/// ```
///
/// A `const` table of unit headings — array plumbing, not a domain value (the
/// no-bare-types carve-out); each entry is a screen-space unit XY direction.
pub const COMPASS_DIRECTIONS: [Vec2; DIRECTION_COUNT] = {
    // 1/sqrt(2), the diagonal unit component (const-evaluable literal — `f32::sqrt`
    // is not const). Keeps every entry unit length so the nearest-pick is a clean
    // dot-product compare.
    const D: f32 = 0.707_106_77;
    [
        Vec2::new(1.0, 0.0),  // 0 E  — heading screen +x
        Vec2::new(D, -D),     // 1 SE — +x, screen -y (down)
        Vec2::new(0.0, -1.0), // 2 S  — screen -y
        Vec2::new(-D, -D),    // 3 SW
        Vec2::new(0.0, 1.0),  // 4 N  — screen +y (up)
        Vec2::new(-D, D),     // 5 NW
        Vec2::new(-1.0, 0.0), // 6 W  — screen -x
        Vec2::new(D, D),      // 7 NE
    ]
};

/// One damage type's directional-shot + impact FX tiles — the per-row payload of
/// the per-damage-type FX model.
///
/// Each authored row of `assets/tiles/alt_tileset_effects.png` (cols 0..8 the
/// 8-way directional projectile rose, cols 8..11 the 3-frame impact animation)
/// resolves into one of these: [`directions`](DamageTypeFx::directions) indexes
/// the directional projectile tile by [`COMPASS_DIRECTIONS`] column, and
/// [`impact`](DamageTypeFx::impact) is the 3-frame impact sequence. Every index is
/// data the engineer eyeballs against the sheet — nothing is hardcoded in Rust.
///
/// Derives [`Deserialize`] (the authored `.ron` shape) + value traits; it is NOT a
/// `Resource` (it is a FIELD of the [`EffectRoles`] resource), and NOT a
/// `Component`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DamageTypeFx {
    /// The 8 directional projectile tiles, indexed by [`COMPASS_DIRECTIONS`] column
    /// (`0 E .. 7 NE`) — the sprite the traveling projectile shows for the shot's
    /// nearest heading ([`nearest_direction_index`]).
    pub directions: [TileIndex; DIRECTION_COUNT],
    /// The 3 impact-animation frames in play order (solid burst -> open ring ->
    /// breaking ring) — the expanding-shockwave sequence FX-B's `animate_impact`
    /// steps through at the projectile's arrival point.
    pub impact:     [TileIndex; IMPACT_FRAME_COUNT],
}

/// The DATA-DRIVEN FX tile-role table — the per-damage-type projectile/impact FX
/// plus the three legacy consequence-flash tiles.
///
/// Loaded from the loose `assets/tiles/effect_roles.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`EffectRoles`] resource before battle time (see [`resolve_effect_roles`]),
/// mirroring S4's `tile_roles.ron` / `load_tile_roles` / `resolve_tile_roles`.
///
/// GTW-306 reshaped the firing-FX half from the old single stretched-`tracer`
/// index into a PER-DAMAGE-TYPE model: each of the four authored color rows
/// ([`orange`](EffectRoles::orange) / [`blue`](EffectRoles::blue) /
/// [`green`](EffectRoles::green) / [`purple`](EffectRoles::purple)) holds an
/// 8-way directional projectile rose + a 3-frame impact animation
/// ([`DamageTypeFx`]); [`fx_for`](EffectRoles::fx_for) maps a sim [`DamageType`]
/// onto one of those rows (with [`fallback`](EffectRoles::fallback) for any type
/// the sheet has no dedicated row for). The legacy [`bleed`](EffectRoles::bleed) /
/// [`armor_break`](EffectRoles::armor_break) /
/// [`cover_destroyed`](EffectRoles::cover_destroyed) consequence-flash tiles (sheet
/// rows 5..8) are unchanged.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<EffectRoles>`](gdtf_assets::RonAsset)
/// requires of its payload).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct EffectRoles {
    /// The [`Bleeding`](gdtf_battle_sim::Bleeding) blood/hit-flash tile (the §9 bleed-out signal).
    pub bleed:           TileIndex,
    /// The [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) spark/break-burst tile (the shattered-piece signal).
    pub armor_break:     TileIndex,
    /// The [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) debris/rubble-burst tile (the §3 cover-smashed signal).
    pub cover_destroyed: TileIndex,
    /// The ORANGE damage-type row (sheet row 0) — its 8-way projectile rose + 3-frame impact.
    pub orange:          DamageTypeFx,
    /// The BLUE damage-type row (sheet row 1) — its 8-way projectile rose + 3-frame impact.
    pub blue:            DamageTypeFx,
    /// The GREEN damage-type row (sheet row 2) — its 8-way projectile rose + 3-frame impact.
    pub green:           DamageTypeFx,
    /// The PURPLE damage-type row (sheet row 3) — its 8-way projectile rose + 3-frame impact.
    pub purple:          DamageTypeFx,
}

impl EffectRoles {
    /// The per-damage-type projectile/impact FX row for `damage`.
    ///
    /// Maps each sim [`DamageType`] wheel node onto one of the four authored color
    /// rows by visual fit (`docs/combat/matchup.md` §"The 7 types" names the nodes;
    /// the sheet ships only four color variants for seven types, so several types
    /// share a row): warm slugs/blast read ORANGE, beams/arc read BLUE, toxin/gas
    /// reads GREEN, superheated/power-edge reads PURPLE. Any future type the sheet
    /// gains no dedicated row for still resolves through [`fallback`](EffectRoles::fallback).
    /// Returns a reference so the caller (`projectile` / `impact`) reads the tiles
    /// without cloning the arrays.
    #[must_use]
    pub const fn fx_for(&self, damage: DamageType) -> &DamageTypeFx {
        match damage {
            // Kinetic (slugs/autoguns) + Blast (explosives) — warm orange.
            DamageType::Kinetic | DamageType::Blast => &self.orange,
            // Las (beams) + Shock (arc/EMP) — cool blue.
            DamageType::Las | DamageType::Shock => &self.blue,
            // Chem (toxin/acid/gas) — green.
            DamageType::Chem => &self.green,
            // Plasma (superheated) + Rend (chain/power edges) — purple.
            DamageType::Plasma | DamageType::Rend => &self.purple,
        }
    }

    /// The FALLBACK projectile/impact FX row — the ORANGE row.
    ///
    /// The sensible default for any damage type the sheet has no dedicated row for
    /// (the SIM handoff: today only Kinetic spawns, but the per-type MECHANISM must
    /// degrade gracefully). [`fx_for`](EffectRoles::fx_for) is total over the seven
    /// CURRENT variants, so this is the explicit fallback the contract requires for
    /// the not-yet-in-data types — orange being the warm slug-shot read.
    #[must_use]
    pub const fn fallback(&self) -> &DamageTypeFx {
        &self.orange
    }
}

/// The directional-strip COLUMN index whose [`COMPASS_DIRECTIONS`] heading best
/// matches `trajectory`'s XY direction — the tile the traveling projectile shows.
///
/// Projects the shot's 3D [`trajectory`](gdtf_battle_sim::ShotFired::trajectory)
/// onto the world-screen XY plane (the sim `+y` cell axis maps to screen `+y`, the
/// frame [`COMPASS_DIRECTIONS`] lives in) and picks the compass column whose unit
/// heading has the LARGEST dot product with it — i.e. the nearest of the 8
/// directions. A degenerate (near-zero XY) trajectory — a straight-up/down shot —
/// has no meaningful heading, so it falls back to column `0` (E) rather than
/// picking arbitrarily. Returns a column index in `0..`[`DIRECTION_COUNT`].
#[must_use]
pub fn nearest_direction_index(trajectory: Vec3) -> usize {
    // The shot's heading in the screen XY frame (drop z; the rose is 2D).
    let heading = trajectory.truncate();
    if heading.length_squared() <= f32::EPSILON {
        // Straight up/down — no XY heading; default to the first column (E).
        return 0;
    }
    let heading = heading.normalize_or_zero();
    let mut best_index = 0;
    let mut best_dot = f32::NEG_INFINITY;
    for (index, dir) in COMPASS_DIRECTIONS.iter().enumerate() {
        let dot = heading.dot(*dir);
        if dot > best_dot {
            best_dot = dot;
            best_index = index;
        }
    }
    best_index
}

/// The path of the loose FX-role RON, relative to the asset source root.
const EFFECT_ROLES_RON_PATH: &str = "tiles/effect_roles.ron";

/// The in-flight handle to the FX-role RON, held until it resolves into [`EffectRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no domain
/// meaning; this name says "the FX-role table being loaded"). Inserted by
/// [`load_effect_roles`] and read by [`resolve_effect_roles`], mirroring S4's
/// `TileRolesHandle`.
#[derive(Resource, Deref, Debug, Clone)]
pub struct EffectRolesHandle(Handle<RonAsset<EffectRoles>>);

impl EffectRolesHandle {
    /// Wrap the in-flight FX-role RON handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<EffectRoles>>) -> Self {
        Self(handle)
    }
}

/// `Startup`: kick off the `effect_roles.ron` load, storing its typed handle.
///
/// Loads `tiles/effect_roles.ron` as a [`RonAsset<EffectRoles>`](gdtf_assets::RonAsset)
/// through the generic GTW-136 loader and inserts the [`EffectRolesHandle`] the
/// [`resolve_effect_roles`] poll system reads. Takes `Option<Res<AssetServer>>` so a
/// `MinimalPlugins` headless app with no [`AssetServer`] no-ops rather than panicking
/// (`bevy-traps.md` #1); under `DefaultPlugins` the load fires for real.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_effect_roles(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<EffectRoles>>(EFFECT_ROLES_RON_PATH);
    commands.insert_resource(EffectRolesHandle::new(handle));
}

/// `Update` (gated until [`EffectRoles`] is resolved): resolve the loaded RON into the
/// presenter-owned [`EffectRoles`] resource.
///
/// Once the [`RonAsset<EffectRoles>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<EffectRoles>>`, it clones the deserialized [`EffectRoles`] out and inserts
/// it as the resident resource so it is present before the first FX message. Run only while
/// [`EffectRolesHandle`] exists AND [`EffectRoles`] does NOT (the plugin's run-condition), so
/// it inserts once. Mirrors S4's `resolve_tile_roles`.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert, [`Res<EffectRolesHandle>`]
/// for the handle, [`Res<Assets<RonAsset<EffectRoles>>>`] for the loaded asset.
pub fn resolve_effect_roles(
    mut commands: Commands,
    handle: Res<EffectRolesHandle>,
    roles_assets: Res<Assets<RonAsset<EffectRoles>>>,
) {
    let Some(loaded) = roles_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until EffectRoles is resolved.
        return;
    };
    commands.insert_resource((**loaded).clone());
}

/// `Update` (unguarded; self-gates on its [`Option`] borrows): live-reload the resident
/// [`EffectRoles`] resource when its `effect_roles.ron` is re-saved (GTW-375 C5).
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`EffectRoles`]`>>` — asset
/// events are MESSAGES in Bevy 0.19, so this is a `MessageReader`, not an `EventReader`
/// (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the in-flight
/// [`EffectRolesHandle`]; events for any other handle are ignored. On a match it clones the
/// refreshed [`EffectRoles`] out of the `Assets` collection and overwrites the resident
/// resource through [`ResMut`]. Mirrors
/// [`redrive_tile_roles_on_asset_event`](crate::redrive_tile_roles_on_asset_event).
///
/// # Why no existing-flash re-render is needed (the C5 transience clause)
///
/// FX flashes (the consequence flashes — [`read_bleeding`](super::read_bleeding) /
/// [`read_armor_broken`](super::read_armor_broken) /
/// [`read_cover_destroyed`](super::read_cover_destroyed)) and shot projectiles
/// ([`spawn_shot_projectiles`](super::spawn_shot_projectiles) /
/// [`animate_impact`](super::animate_impact)) are TRANSIENT: each is spawned per sim event
/// from the CURRENT [`EffectRoles`] indices, then expires on a timer
/// ([`expire_flashes`](super::expire_flashes)) / despawns at impact. The flash / projectile
/// readers read [`EffectRoles`] at SPAWN time, so once this redrive overwrites the resource,
/// every FUTURE flash uses the new indices automatically — there is NO persistent FX sprite
/// to re-index, so this redrive's re-resolve is sufficient (unlike the ganger sprites, which
/// persist and DO need [`reindex_ganger_sprites_on_character_roles_change`]).
///
/// [`EffectRoles`] is NOT `Copy` (its nested [`DamageTypeFx`] arrays are not `Copy`), so the
/// overwrite is `(**updated).clone()`.
///
/// Guarded so it never panics before the load chain has run (pre-resolve): it takes the
/// handle / the `Assets` collection / the [`EffectRoles`] resource as [`Option`]al borrows,
/// draining the reader and returning early if any is missing (`bevy-traps.md` #1) so a
/// pre-resolve event does not linger and re-fire later.
///
/// [`reindex_ganger_sprites_on_character_roles_change`]: crate::reindex_ganger_sprites_on_character_roles_change
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle / `Assets` /
/// [`EffectRoles`] borrows.
pub fn redrive_effect_roles_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<EffectRoles>>>,
    handle: Option<Res<EffectRolesHandle>>,
    roles_assets: Option<Res<Assets<RonAsset<EffectRoles>>>>,
    roles: Option<ResMut<EffectRoles>>,
) {
    let (Some(handle), Some(roles_assets), Some(mut roles)) = (handle, roles_assets, roles) else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to re-resolve yet.
        events.clear();
        return;
    };

    let active_id = handle.id();
    // Act once per frame even if several Modified events arrive: a single re-resolve from
    // the latest in-memory value covers them all.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = roles_assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload state) — leave
        // the existing roles until it settles; the next event re-fires.
        return;
    };
    // EffectRoles derives Clone (NOT Copy), so clone the refreshed table out of the asset.
    *roles = (**updated).clone();
    // GTW-374 Part C convention: log EVERY hot-reload path naming what reloaded, so a live
    // effect-role edit can be traced (mirrors the tile / character / combat / theme handlers).
    info!("effect hot-reload: re-resolved EffectRoles from `tiles/effect_roles.ron`");
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use bevy::{
        MinimalPlugins,
        asset::{AssetEvent, AssetPlugin, Assets, Handle},
        ecs::system::RunSystemOnce,
        log::{
            tracing::{
                Event, Subscriber,
                field::{Field, Visit},
                subscriber::with_default,
            },
            tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
        },
        prelude::*,
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};

    use super::{
        DIRECTION_COUNT, DamageTypeFx, EffectRoles, EffectRolesHandle, IMPACT_FRAME_COUNT,
        redrive_effect_roles_on_asset_event,
    };
    use crate::TileIndex;

    /// A scoped `tracing` layer that records each event's `message` field — the minimal
    /// capture needed to prove the `info!` hot-reload line fired. Mirrors the GTW-374
    /// combat redrive test's / `terrain/roles.rs`'s `CaptureLayer` (no shared util is
    /// reachable from this crate).
    struct CaptureLayer {
        /// The shared buffer captured messages append to.
        messages: Arc<Mutex<Vec<String>>>,
    }

    /// Pulls the `message` field's debug rendering out of a `tracing` event.
    struct MessageVisitor {
        /// The captured message text, if a `message` field was visited.
        message: Option<String>,
    }

    impl Visit for MessageVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = Some(format!("{value:?}"));
            }
        }
    }

    impl<S: Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = MessageVisitor { message: None };
            event.record(&mut visitor);
            if let Some(message) = visitor.message
                && let Ok(mut buffer) = self.messages.lock()
            {
                buffer.push(message);
            }
        }
    }

    /// Run `body` with a scoped [`CaptureLayer`] active, returning every captured message.
    ///
    /// The subscriber is scoped to this call (`with_default`), so it never leaks into other
    /// tests; the returned `Vec` is in emission order.
    fn capture_logs(body: impl FnOnce()) -> Vec<String> {
        let messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let layer = CaptureLayer {
            messages: Arc::clone(&messages),
        };
        let subscriber = Registry::default().with(layer);
        with_default(subscriber, body);
        messages
            .lock()
            .map(|buffer| buffer.clone())
            .unwrap_or_default()
    }

    /// A headless app with the real effect-role hot-reload wiring: `MinimalPlugins` +
    /// `AssetPlugin` (so `Assets<RonAsset<EffectRoles>>` and the `AssetEvent` message buffer
    /// exist), the `EffectRoles` RON loader registered, and the redrive system in `Update`.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset::<EffectRoles>()
            .add_systems(Update, redrive_effect_roles_on_asset_event);
        app
    }

    /// A `DamageTypeFx` row whose every directional + impact tile is `seed` — a valid
    /// fixture; the redrive only re-resolves the whole table, it never reads a single tile.
    fn uniform_fx(seed: usize) -> DamageTypeFx {
        DamageTypeFx {
            directions: [TileIndex::new(seed); DIRECTION_COUNT],
            impact:     [TileIndex::new(seed); IMPACT_FRAME_COUNT],
        }
    }

    /// An `EffectRoles` whose `bleed` index is `bleed` and every other field is derived
    /// from a fixed seed — the distinct fixtures the re-resolve test contrasts.
    fn roles_with_bleed(bleed: usize) -> EffectRoles {
        EffectRoles {
            bleed:           TileIndex::new(bleed),
            armor_break:     TileIndex::new(1),
            cover_destroyed: TileIndex::new(2),
            orange:          uniform_fx(10),
            blue:            uniform_fx(11),
            green:           uniform_fx(12),
            purple:          uniform_fx(13),
        }
    }

    /// Add a `RonAsset<EffectRoles>` to the collection and return its handle.
    fn add_asset(app: &mut App, roles: EffectRoles) -> Handle<RonAsset<EffectRoles>> {
        app.world_mut()
            .resource_mut::<Assets<RonAsset<EffectRoles>>>()
            .add(RonAsset(roles))
    }

    /// Overwrite the in-memory payload of an already-added effect-role asset (the hot edit
    /// the file-watcher would make on an `effect_roles.ron` save).
    fn hot_edit(app: &mut App, handle: &Handle<RonAsset<EffectRoles>>, roles: EffectRoles) {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<EffectRoles>>>();
        if let Some(mut asset) = assets.get_mut(handle) {
            asset.0 = roles;
        }
    }

    /// Inject an `AssetEvent::Modified` for the given handle id (standing in for the
    /// file-watcher's reload signal).
    fn inject_modified(app: &mut App, handle: &Handle<RonAsset<EffectRoles>>) {
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
    }

    /// C10(d): on a matching `Modified` for the active effect-role handle, the redrive
    /// system re-resolves the resident `EffectRoles` resource to the UPDATED in-memory
    /// payload — the live effect hot-reload. Driven through the real registered system via
    /// `app.update()`.
    ///
    /// Pin-discriminating: dropping the re-resolve leaves `EffectRoles` on the OLD `bleed`
    /// index; a wrong id filter would re-resolve on any handle's event.
    #[test]
    fn modified_event_reresolves_effect_roles() {
        let mut app = app();

        let baseline = roles_with_bleed(5);
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(EffectRolesHandle::new(handle.clone()));

        // First update: no event, the resource is untouched.
        app.update();

        // Hot-edit the asset to a DISTINCT table, then fire a Modified.
        let edited = roles_with_bleed(77);
        assert_ne!(edited, baseline, "precondition: the edit must differ");
        hot_edit(&mut app, &handle, edited.clone());
        inject_modified(&mut app, &handle);
        app.update();

        assert_eq!(
            app.world().get_resource::<EffectRoles>(),
            Some(&edited),
            "a Modified for the active effect-role handle must re-resolve EffectRoles \
             to the new values",
        );
    }

    /// C10(d): a `Modified` for a DIFFERENT asset id leaves `EffectRoles` untouched — the
    /// filter is on the ACTIVE handle id only.
    ///
    /// Pin-discriminating: dropping the id filter re-resolves on any effect-role asset's
    /// event.
    #[test]
    fn modified_event_for_other_id_does_not_reresolve() {
        let mut app = app();

        let baseline = roles_with_bleed(5);
        let active = add_asset(&mut app, baseline.clone());
        // A second, unrelated effect-role asset whose value differs from the active one.
        let other = add_asset(&mut app, roles_with_bleed(88));
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(EffectRolesHandle::new(active));

        app.update();
        // Edit the OTHER asset, fire Modified for it only.
        hot_edit(&mut app, &other, roles_with_bleed(33));
        inject_modified(&mut app, &other);
        app.update();

        assert_eq!(
            app.world().get_resource::<EffectRoles>(),
            Some(&baseline),
            "a Modified for a non-active effect-role id must NOT re-resolve EffectRoles",
        );
    }

    /// C7: the hot-reload `info!` line FIRES on the real re-resolve path, naming what
    /// reloaded. The redrive system is run via `run_system_once` on the calling thread
    /// (inside the scoped `tracing` subscriber) so the capture — which is thread-local —
    /// sees the emission (GTW-374 thread-local capture lesson).
    ///
    /// Pin-discriminating: removing the `info!` from the redrive leaves the capture empty
    /// and this assert fails.
    #[test]
    fn hot_reload_logs_an_info_line() {
        let mut app = app();
        let baseline = roles_with_bleed(5);
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline);
        app.world_mut()
            .insert_resource(EffectRolesHandle::new(handle.clone()));
        // Stage the hot edit + the Modified message, then run the redrive synchronously
        // inside the capture scope.
        hot_edit(&mut app, &handle, roles_with_bleed(77));
        inject_modified(&mut app, &handle);

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_effect_roles_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("effect hot-reload") && line.contains("EffectRoles")),
            "the effect-role hot-reload must emit an info! line naming what reloaded; \
             captured: {captured:?}",
        );
    }
}
