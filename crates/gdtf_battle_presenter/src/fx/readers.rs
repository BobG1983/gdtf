//! The three sim-FX-message readers and their shared sprite/spawn/tint helpers.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{
    ArmorBroken, Bleeding, Cell, CoverDestroyed, Level, Position, ShotFired, Wounds,
};

use super::{
    flash::{FlashTtl, FxFlash},
    roles::EffectRoles,
};
use crate::{CELL_PX, SheetRole, TileIndex, TopDownAtlases, cell_to_world, sim_pos_to_world};

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
pub(super) fn bleed_tint(wounds: Wounds) -> Color {
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

/// Spawn one transient TRACER-beam flash from `muzzle` to `impact` (both world-space)
/// showing `sprite` — a single effects sprite STRETCHED to the muzzle→impact length and
/// ROTATED to its angle, on a [`FlashTtl`] lifetime ([`FxFlash`]-marked like every flash).
///
/// The beam sits at the muzzle→impact MIDPOINT, rotated by the segment's XY angle, and
/// scaled along its local +X by `length / `[`CELL_PX`] so the base `CELL_PX`-square sprite
/// covers the full distance (its +Y stays at the sprite's own width so the beam reads as a
/// thin line). A degenerate zero-length segment (muzzle == impact) draws nothing rather
/// than dividing by zero. The beam's `z` is the muzzle's (the shot draws in the firing
/// storey's band). Mirrors [`spawn_flash`]'s bundle, differing only in the stretch/rotate
/// transform the muzzle/impact flashes do not need.
fn spawn_tracer(commands: &mut Commands, sprite: Sprite, muzzle: Vec3, impact: Vec3) {
    let delta = (impact - muzzle).truncate(); // the XY segment the tracer spans
    let length = delta.length();
    if length <= f32::EPSILON {
        // Zero-length segment (muzzle coincides with impact) — no beam to draw.
        return;
    }
    let midpoint = muzzle.lerp(impact, 0.5);
    let angle = delta.y.atan2(delta.x); // the muzzle->impact heading in the XY plane
    let mut transform = Transform::from_translation(midpoint.with_z(muzzle.z));
    transform.rotation = Quat::from_rotation_z(angle);
    // Stretch the CELL_PX-square sprite along its local +X to span the full distance; the
    // local +Y keeps the sprite's own width so the beam reads as a thin line.
    transform.scale = Vec3::new(length / CELL_PX, 1.0, 1.0);
    commands.spawn((
        sprite,
        transform,
        RenderLayers::layer(crate::WORLD_RENDER_LAYER),
        FlashTtl::new(),
        FxFlash,
    ));
}

/// `Update` (`PresenterSystems::Draw`): spawn the GTW-290 muzzle / tracer / impact FX per
/// [`ShotFired`] message.
///
/// Drains [`MessageReader<ShotFired>`](gdtf_battle_sim::ShotFired); for each round fired it
/// spawns THREE transient [`FlashTtl`] flashes from the message's already-resolved sim
/// geometry (no rule logic — the presenter only draws what the sim emitted):
///
/// - a **muzzle flash** (the table's `muzzle_flash` index) at the muzzle world position
///   ([`sim_pos_to_world`](crate::sim_pos_to_world) of `msg.muzzle`);
/// - a **tracer** (the `tracer` index) stretched + rotated along muzzle→impact
///   ([`spawn_tracer`]); and
/// - a generic **impact** mark (the `impact` index) at the impact world position
///   ([`cell_to_world`](crate::cell_to_world) of `msg.impact_cell` / `msg.impact_level`).
///
/// This is the GENERIC projectile FX ONLY — it does NOT duplicate the
/// [`Bleeding`](gdtf_battle_sim::Bleeding) / [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) /
/// [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) CONSEQUENCE flashes ([`read_bleeding`]
/// / [`read_armor_broken`] / [`read_cover_destroyed`] draw those off their own sim
/// messages). A miss still draws muzzle + tracer terminating at the impact cell + the
/// generic impact mark. A burst emits one [`ShotFired`] per round, so it draws one tracer
/// per round. Every index is the table's (never a literal); the muzzle/impact tints are an
/// opaque white spark, leaving colour to the sheet art. A missing effects sheet skips the
/// spawn fail-closed (`fx_sprite` returns [`None`]).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`],
/// [`Res<EffectRoles>`], and [`MessageReader<ShotFired>`].
pub fn read_shot_fired(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut shots: MessageReader<ShotFired>,
) {
    for msg in shots.read() {
        let muzzle_world = sim_pos_to_world(msg.muzzle);
        let impact_world = cell_to_world(msg.impact_cell, msg.impact_level);

        // (1) Muzzle flash at the 3D fire origin.
        if let Some(sprite) = fx_sprite(roles.muzzle_flash, Color::WHITE, &atlases) {
            spawn_flash(&mut commands, sprite, muzzle_world);
        }
        // (2) Tracer beam stretched + rotated along muzzle -> impact.
        if let Some(sprite) = fx_sprite(roles.tracer, Color::WHITE, &atlases) {
            spawn_tracer(&mut commands, sprite, muzzle_world, impact_world);
        }
        // (3) Generic impact mark at the impact cell (NOT a consequence FX).
        if let Some(sprite) = fx_sprite(roles.impact, Color::WHITE, &atlases) {
            spawn_flash(&mut commands, sprite, impact_world);
        }
    }
}
