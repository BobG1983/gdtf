//! Transient FX flashes (GTW-48 S6 / GTW-220): the presenter's one-shot FX layer.
//!
//! This module turns the three already-landed sim FX MESSAGES into short-lived 16x16
//! FX sprites drawn from the effects sheet ([`SheetRole::Effects`](crate::SheetRole),
//! `assets/tiles/alt_tileset_effects.png`):
//!
//! - [`Bleeding`](gdtf_battle_sim::Bleeding) `{ ganger }` -> a blood/hit FLASH sprite at
//!   the ganger's cell ([`read_bleeding`]); the flash's intensity is a RELATION to the
//!   ganger's [`Wounds`](gdtf_battle_sim::Wounds) (the message carries NO amount — verified
//!   `bleed.rs:62-65`), read from a `Query<&Wounds>`, never a pinned literal.
//! - [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) `{ ganger, part }` -> a spark/break flash
//!   at the ganger's cell ([`read_armor_broken`]).
//! - [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) `{ at }` -> a debris/rubble burst at
//!   `cell_to_world(at)` ([`read_cover_destroyed`]); ADDITIVE to the S4 rubble swap.
//!
//! Every spawned flash carries a [`FlashTtl`] lifetime + an [`FxFlash`] marker; the
//! [`expire_flashes`] system ticks each [`FlashTtl`] with [`Res<Time>`] and despawns the
//! flash on expiry — THAT expiry is what makes the flashes one-shot / transient. Multiple
//! messages for the same ganger/cell in one frame each spawn an INDEPENDENT short-lived
//! sprite (NO coalescing this slice).
//!
//! WHICH effect tile each FX draws is DATA-DRIVEN: a per-line-commented
//! `assets/tiles/effect_roles.ron`, loaded through the SAME generic
//! [`RonAsset<T>`](gdtf_assets::RonAsset) loader S4's `tile_roles.ron` uses, mapping each FX
//! to a [`TileIndex`](crate::TileIndex). Nothing about the index choices is hardcoded in
//! Rust.
//!
//! It only READS the three sim messages (+ a `Query<&Position>` / `Query<&Wounds>`) and adds
//! ZERO sim setup/teardown. It mirrors, never owns, combat truth — the one-way
//! `input -> presenter -> sim` edge (ADR-0001); the sim never reads the presenter.

use std::time::Duration;

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{ArmorBroken, Bleeding, Cell, CoverDestroyed, Level, Position, Wounds};
use serde::Deserialize;

use crate::{CELL_PX, SheetRole, TileIndex, TopDownAtlases, cell_to_world};

/// How long one transient FX flash stays on screen, in seconds.
///
/// A small, fixed pop-and-fade window so a flash reads as a momentary burst rather than a
/// persistent sprite. A `const`, NOT a domain newtype — the framework-plumbing carve-out
/// (`.claude/rules/no-bare-types.md` clause 4): a scalar fed straight to a [`Timer`]
/// duration, the same reasoning the landed `CELL_PX`-class consts use. The [`FlashTtl`]
/// domain VALUE (the live countdown) IS a newtype.
const FLASH_SECONDS: f32 = 0.4;

/// Marker tagging every transient FX flash sprite this slice spawns.
///
/// A value-free marker (the no-bare-types marker carve-out) so [`expire_flashes`] finds
/// exactly the FX flashes — and ONLY them, never the S4 [`TerrainSprite`](crate::TerrainSprite),
/// never the S5 [`GangerSprite`](crate::GangerSprite), never the S2
/// [`WorldCamera`](crate::WorldCamera).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FxFlash;

/// The lifetime of one transient FX flash — its despawn clock.
///
/// A NAMED newtype over a [`Timer`] (no-bare-types: a transient-FX lifetime is a domain
/// value, not a bare `Timer`), [`Deref`]ing to it so a reader inspects the timer straight
/// through. THIS countdown is what makes "one-shot" observable: [`expire_flashes`] advances
/// it each `Update` and despawns the flash the moment it [`Timer::finished`]. A fresh
/// [`FlashTtl::new`] starts a [`FLASH_SECONDS`] one-shot ([`TimerMode::Once`]) clock.
#[derive(Component, Deref, Debug, Clone)]
pub struct FlashTtl(Timer);

impl FlashTtl {
    /// Start a fresh one-shot flash clock running for [`FLASH_SECONDS`].
    ///
    /// [`TimerMode::Once`] so the timer finishes exactly once (it does not loop), which is
    /// the "one-shot" semantics [`expire_flashes`] keys its despawn on.
    #[must_use]
    pub fn new() -> Self {
        Self(Timer::from_seconds(FLASH_SECONDS, TimerMode::Once))
    }

    /// Advance this flash clock by `delta` and report whether it has now expired.
    ///
    /// Wraps [`Timer::tick`] + [`Timer::is_finished`] so the inner [`Timer`] is mutated
    /// through a named method (no `DerefMut` exposed — the lifetime is advanced ONLY here).
    /// Returns `true` once the [`FLASH_SECONDS`] window has elapsed (a [`TimerMode::Once`]
    /// timer stays finished once it crosses), the signal [`expire_flashes`] despawns on.
    pub fn tick(&mut self, delta: Duration) -> bool {
        self.0.tick(delta).is_finished()
    }
}

impl Default for FlashTtl {
    /// A fresh [`FLASH_SECONDS`] one-shot clock — same as [`FlashTtl::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// The DATA-DRIVEN FX tile-role table — each one-shot FX -> its [`TileIndex`].
///
/// Loaded from the loose `assets/tiles/effect_roles.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`EffectRoles`] resource before battle time (see [`resolve_effect_roles`]), mirroring
/// S4's `tile_roles.ron` / `load_tile_roles` / `resolve_tile_roles`. Every index is data the
/// engineer eyeballs against the effects sheet and may adjust — nothing about the index
/// choices is hardcoded in Rust; this struct only names the FX ROLES, each indexing the 16x8
/// [`SheetRole::Effects`](crate::SheetRole) sheet.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored `.ron`
/// shape), and [`TypePath`] (the bound [`RonAsset<EffectRoles>`](gdtf_assets::RonAsset)
/// requires of its payload).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct EffectRoles {
    /// The [`Bleeding`](gdtf_battle_sim::Bleeding) blood/hit-flash tile (the §9 bleed-out signal).
    pub bleed:           TileIndex,
    /// The [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) spark/break-burst tile (the shattered-piece signal).
    pub armor_break:     TileIndex,
    /// The [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) debris/rubble-burst tile (the §3 cover-smashed signal).
    pub cover_destroyed: TileIndex,
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
pub struct EffectRolesHandle(pub Handle<RonAsset<EffectRoles>>);

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
    commands.insert_resource(EffectRolesHandle(handle));
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

/// The world `(cell, level)` a ganger's [`Position`] projects to — reconstruct the typed
/// [`Cell`] / [`Level`] from the position's `IVec3` components (the S4/S5 `cell_and_level`
/// idiom), since [`Position`] Derefs to [`CellLevel`](gdtf_battle_sim::CellLevel) Derefs to
/// `IVec3`.
///
/// `pos.z` is a storey index in `0..MAX_LEVELS`; clamping the (impossible-in-practice)
/// negative / over-`u8` case keeps the reconstruction panic-free.
fn cell_and_level(pos: &Position) -> (Cell, Level) {
    let cell = Cell::new(pos.x, pos.y);
    let storey = u8::try_from(pos.z).unwrap_or(0);
    (cell, Level::new(storey))
}

/// The bleed-flash tint, as a relation to the bleeding ganger's remaining [`Wounds`].
///
/// The [`Bleeding`](gdtf_battle_sim::Bleeding) message carries NO amount (verified
/// `bleed.rs:62-65`), so the flash's intensity is read STRUCTURALLY from `*Wounds`, never a
/// pinned literal and never from the message: a ganger nearer death (FEWER remaining
/// [`Wounds`]) bleeds a more saturated, opaque red, while one with wounds to spare reads
/// fainter. Alpha rises as remaining wounds fall (`alpha = 1 / (1 + *Wounds)`, clamped into a
/// visible floor), so a freshly-Downed ganger's bleed flash is brighter the closer it is to
/// the lethal tick — a relation to the live pool, recomputed each message.
fn bleed_tint(wounds: Wounds) -> Color {
    // alpha is a strictly-decreasing relation to remaining wounds: more wounds left => a
    // fainter flash; near-empty => near-opaque. f32::from (not `as`) for the lossless widen.
    let remaining = f32::from(*wounds);
    let alpha = (1.0 / (1.0 + remaining)).clamp(0.35, 1.0);
    Color::srgba(0.8, 0.05, 0.05, alpha)
}

/// Builds one FX [`Sprite`] on the effects sheet at `index`, tinted `tint`, via the S3 recipe.
///
/// `Sprite::from_atlas_image(effects.image, TextureAtlas { layout, index })` with
/// `custom_size = Some(Vec2::splat(CELL_PX))` (the documented S3 sizing recipe) and the `tint`
/// applied to `Sprite.color`. Returns [`None`] if the effects sheet was not loaded (so the
/// caller skips the spawn rather than panic).
fn fx_sprite(index: TileIndex, tint: Color, atlases: &TopDownAtlases) -> Option<Sprite> {
    let effects = atlases.role(SheetRole::Effects)?;
    let mut sprite = Sprite::from_atlas_image(
        effects.image.clone(),
        TextureAtlas {
            layout: effects.layout.clone(),
            index:  *index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX));
    sprite.color = tint;
    Some(sprite)
}

/// The shared spawn bundle for one transient FX flash at `world` showing `sprite`.
///
/// Every FX flash is the same shape: the effects [`Sprite`], a [`Transform`] at the cell's
/// world position, the [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), a fresh
/// [`FlashTtl`] one-shot clock, and the [`FxFlash`] marker. Factored so the three readers
/// spawn identically (the only per-FX difference is the sprite's index + tint).
fn spawn_flash(commands: &mut Commands, sprite: Sprite, world: Vec3) {
    commands.spawn((
        sprite,
        Transform::from_translation(world),
        RenderLayers::layer(crate::WORLD_RENDER_LAYER),
        FlashTtl::new(),
        FxFlash,
    ));
}

/// `Update` (`PresenterSystems::Draw`): spawn a blood/hit FX flash per [`Bleeding`] message.
///
/// Drains [`MessageReader<Bleeding>`](gdtf_battle_sim::Bleeding); for each `Bleeding { ganger }`
/// it looks up the ganger's cell via `Query<&Position>.get(msg.ganger)` (reconstructing the
/// typed [`Cell`] / [`Level`] via [`cell_and_level`]) and its remaining
/// [`Wounds`](gdtf_battle_sim::Wounds) via `Query<&Wounds>.get(msg.ganger)`, then spawns ONE
/// FX flash at [`cell_to_world`](crate::cell_to_world) carrying [`FlashTtl`] + [`FxFlash`].
/// The flash's index is the table's `bleed` [`TileIndex`] (never a literal); its tint is a
/// RELATION to `*Wounds` ([`bleed_tint`]) — the [`Bleeding`] message carries NO amount. A
/// ganger missing its [`Position`] OR its [`Wounds`] is skipped FAIL-CLOSED (`get(..)` is
/// `Err`, the loop `continue`s — no panic, no flash). NO coalescing: two `Bleeding` for one
/// ganger in a frame spawn two independent flashes.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`],
/// [`Res<EffectRoles>`], [`MessageReader<Bleeding>`], and the `Position` / `Wounds` lookups.
pub fn read_bleeding(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut bleeds: MessageReader<Bleeding>,
    positions: Query<&Position>,
    wounds: Query<&Wounds>,
) {
    for msg in bleeds.read() {
        // Fail-closed: no Position OR no Wounds => no flash, no panic.
        let (Ok(pos), Ok(wound)) = (positions.get(msg.ganger), wounds.get(msg.ganger)) else {
            continue;
        };
        let (cell, level) = cell_and_level(pos);
        let Some(sprite) = fx_sprite(roles.bleed, bleed_tint(*wound), &atlases) else {
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}

/// `Update` (`PresenterSystems::Draw`): spawn a spark/break FX flash per [`ArmorBroken`].
///
/// Drains [`MessageReader<ArmorBroken>`](gdtf_battle_sim::ArmorBroken); for each
/// `ArmorBroken { ganger, part }` it looks up the ganger's cell via
/// `Query<&Position>.get(msg.ganger)` and spawns ONE FX flash at
/// [`cell_to_world`](crate::cell_to_world) carrying [`FlashTtl`] + [`FxFlash`], with the
/// table's `armor_break` [`TileIndex`] (never a literal). `msg.part` is available for a
/// per-part tile/tint CHOICE but is not required by an AC; this slice draws a uniform spark
/// burst. A `Position`-less ganger is skipped FAIL-CLOSED.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`],
/// [`Res<EffectRoles>`], [`MessageReader<ArmorBroken>`], and the `Position` lookup.
pub fn read_armor_broken(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut broken: MessageReader<ArmorBroken>,
    positions: Query<&Position>,
) {
    for msg in broken.read() {
        let Ok(pos) = positions.get(msg.ganger) else {
            // Fail-closed: a ganger with no Position spawns no flash, no panic.
            continue;
        };
        let (cell, level) = cell_and_level(pos);
        // A bright opaque spark (the worn piece shattering) — no Wounds relation here.
        let Some(sprite) = fx_sprite(roles.armor_break, Color::WHITE, &atlases) else {
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}

/// `Update` (`PresenterSystems::Draw`): spawn a debris/rubble-burst FX flash per
/// [`CoverDestroyed`].
///
/// Drains [`MessageReader<CoverDestroyed>`](gdtf_battle_sim::CoverDestroyed); for each
/// `CoverDestroyed { at }` it reconstructs the typed [`Cell`] / [`Level`] from `at`
/// ([`CellLevel`](gdtf_battle_sim::CellLevel) Derefs to `IVec3`) and spawns ONE FX flash at
/// `cell_to_world(at)` carrying [`FlashTtl`] + [`FxFlash`], with the table's `cover_destroyed`
/// [`TileIndex`] (never a literal). This is ADDITIVE to the S4 `swap_destroyed_cover` (which
/// swaps the terrain sprite at `at` to rubble) — the transient burst on top, NOT a terrain
/// edit. An off-active-level burst is invisible regardless, so it is not gated on
/// `ActiveLevel`.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`],
/// [`Res<EffectRoles>`], and [`MessageReader<CoverDestroyed>`].
pub fn read_cover_destroyed(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut destroyed: MessageReader<CoverDestroyed>,
) {
    for msg in destroyed.read() {
        // CoverDestroyed.at is a CellLevel; reconstruct its typed Cell / Level (the z is a
        // storey index, clamped panic-free if impossibly out of range).
        let cell = Cell::new(msg.at.x, msg.at.y);
        let storey = u8::try_from(msg.at.z).unwrap_or(0);
        let level = Level::new(storey);
        let Some(sprite) = fx_sprite(roles.cover_destroyed, Color::WHITE, &atlases) else {
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}

/// `Update` (`PresenterSystems::Draw`): the one-shot despawn-on-expiry system.
///
/// Advances each [`FlashTtl`] by the frame [`Res<Time>`] delta ([`FlashTtl::tick`]) and
/// `Commands::entity(e).despawn()`s the flash the moment its clock finishes. THIS is what
/// makes the flashes one-shot / transient — without it, a spawned flash would linger forever.
/// It touches ONLY [`FxFlash`]-marked entities (never terrain / ganger sprites / the camera).
/// Needs no `BattleInProgress` gate — it is inert with no flashes (the query is empty), so it
/// is registered unguarded by that witness.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the despawn, [`Res<Time>`] for the
/// delta, and the `(Entity, &mut FlashTtl)` query (`With<FxFlash>`).
pub fn expire_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut FlashTtl), With<FxFlash>>,
) {
    let delta = time.delta();
    for (entity, mut ttl) in &mut flashes {
        if ttl.tick(delta) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped `effect_roles.ron` parses into `EffectRoles` and exposes every FX role —
    /// a `ron::de` round-trip of the SHIPPED bytes.
    ///
    /// It asserts the file PARSES and HAS all three FX roles (a missing field is a deserialize
    /// error); it does NOT pin a tunable index magnitude (those are data the engineer eyeballs
    /// and may adjust). A light distinctness guard catches an all-collapsed authoring slip.
    #[test]
    fn shipped_effect_roles_ron_parses_with_all_roles() {
        const SHIPPED: &str = include_str!("../../../assets/tiles/effect_roles.ron");
        let parsed: Result<EffectRoles, _> = ron::de::from_str(SHIPPED);
        assert!(
            parsed.is_ok(),
            "shipped effect_roles.ron must parse into EffectRoles, got: {:?}",
            parsed.as_ref().err(),
        );
        let Ok(roles) = parsed else {
            return;
        };
        // The three FX roles must not all collapse onto one index (an authoring slip) — a
        // structural guard, not a magnitude pin.
        let all_same =
            roles.bleed == roles.armor_break && roles.armor_break == roles.cover_destroyed;
        assert!(
            !all_same,
            "the three FX roles must not all share one index (authoring slip)",
        );
    }

    /// `bleed_tint` is a strictly-DECREASING relation in remaining wounds: a ganger nearer
    /// death (fewer wounds) bleeds a more opaque flash, never a pinned literal.
    #[test]
    fn bleed_tint_alpha_decreases_with_remaining_wounds() {
        let near_death = bleed_tint(Wounds::new(0)).alpha();
        let healthier = bleed_tint(Wounds::new(5)).alpha();
        assert!(
            near_death > healthier,
            "fewer remaining wounds must bleed a MORE opaque (higher alpha) flash: \
             {near_death} (0 wounds) must exceed {healthier} (5 wounds)",
        );
    }

    /// A fresh `FlashTtl` is not finished, and ticking it past `FLASH_SECONDS` finishes it —
    /// the one-shot countdown `expire_flashes` keys its despawn on.
    #[test]
    fn flash_ttl_finishes_after_its_window() {
        let mut ttl = FlashTtl::new();
        // A zero tick does not finish a fresh one-shot timer.
        assert!(
            !ttl.tick(Duration::ZERO),
            "a fresh FlashTtl must not be finished before any time passes",
        );
        // Ticking past the full window finishes it.
        let past = Duration::from_secs_f32(FLASH_SECONDS + 0.1);
        assert!(
            ttl.tick(past),
            "ticking a FlashTtl past FLASH_SECONDS must finish it (the one-shot signal)",
        );
    }
}
