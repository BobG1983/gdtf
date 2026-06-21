//! The three sim-FX-message readers and their shared sprite/spawn/tint helpers.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{ArmorBroken, Bleeding, Cell, CoverDestroyed, Level, Position, Wounds};

use super::{
    flash::{FlashTtl, FxFlash},
    roles::EffectRoles,
};
use crate::{CELL_PX, SheetRole, TileIndex, TopDownAtlases, cell_to_world};

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
/// caller skips the spawn rather than panic). `pub(super)` so the GTW-306 `projectile` /
/// `impact` FX systems build their effects sprites from the SAME recipe.
pub(super) fn fx_sprite(index: TileIndex, tint: Color, atlases: &TopDownAtlases) -> Option<Sprite> {
    fx_sprite_scaled(index, tint, 1.0, atlases)
}

/// Builds one FX [`Sprite`] like [`fx_sprite`], but at a UNIFORM `scale` of [`CELL_PX`].
///
/// The sprite's `custom_size` is `Vec2::splat(CELL_PX * scale)` — the SAME factor on BOTH
/// axes, so the tile is enlarged/shrunk uniformly and NEVER stretched along one axis (the
/// GTW-290 smear bug was a one-axis stretch along the shot vector; this is its opposite).
/// GTW-306 readability fix (V2/V3/V4): the small in-tile FX glyphs (the directional comet
/// is only ~7px of its 16px tile, the impact-ring expansion is subtle at 1x) read more
/// clearly when drawn a little larger, and the muzzle pop reads as a tight flash when drawn
/// a little smaller. A `scale` of `1.0` is exactly [`fx_sprite`]. `scale` is framework
/// plumbing (a uniform multiplier fed straight to `custom_size`), not a domain value.
/// `pub(super)` so the `projectile` / `impact` FX systems draw their sprites at their own
/// legible sizes.
pub(super) fn fx_sprite_scaled(
    index: TileIndex,
    tint: Color,
    scale: f32,
    atlases: &TopDownAtlases,
) -> Option<Sprite> {
    let effects = atlases.role(SheetRole::Effects)?;
    let mut sprite = Sprite::from_atlas_image(
        effects.image.clone(),
        TextureAtlas {
            layout: effects.layout.clone(),
            index:  *index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX * scale));
    sprite.color = tint;
    Some(sprite)
}

/// The shared spawn bundle for one transient FX flash at `world` showing `sprite`.
///
/// Every FX flash is the same shape: the effects [`Sprite`], a [`Transform`] at the cell's
/// world position, the [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), a fresh
/// [`FlashTtl`] one-shot clock, and the [`FxFlash`] marker. Factored so the three readers
/// spawn identically (the only per-FX difference is the sprite's index + tint).
///
/// GTW-322 — authored as a `bsn!` scene (the SAME entity tree the old spawn tuple produced,
/// only the spawn SHAPE changed). The atlas-indexed [`Sprite`] is NOT `Unpin` (its
/// `Option<Handle<Image>>` / `Option<TextureAtlas>` fields), so it rides the
/// `template(move |_| Ok(value.clone()))` closure escape hatch (the `FnTemplate` output has
/// no `Unpin` bound), the same one the AREA-1 widget builders + AREA-2 sprite spawns use. The
/// [`Transform`], [`RenderLayers`], and [`FlashTtl`] (a `Timer`-backed newtype) are all
/// `Clone + Default + Unpin`, so each rides [`template_value`] (a value-overwrite). The
/// value-free [`FxFlash`] marker has no `Default` (so no `bsn!` / `template_value` form) — it
/// is `.insert`ed onto the synchronously-reserved id after the scene. `spawn_scene` reserves
/// the id NOW; the scene's components materialize on that frame's `SpawnScene` schedule
/// (between `Update` and `PostUpdate`), so the reader's same-`update()` flash is fully present
/// by `PostUpdate` — identical to the old immediate spawn for the readers' single-update tests.
fn spawn_flash(commands: &mut Commands, sprite: Sprite, world: Vec3) {
    let transform = Transform::from_translation(world);
    let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
    commands
        .spawn_scene((
            bsn! { template(move |_| Ok(sprite.clone())) },
            template_value(transform),
            template_value(layers),
            template_value(FlashTtl::new()),
        ))
        .insert(FxFlash);
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
