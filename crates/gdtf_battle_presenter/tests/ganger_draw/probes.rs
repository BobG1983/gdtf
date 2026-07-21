//! Cross-surface sprite readback probes shared by the `ganger_draw` concern modules.

use bevy::{
    app::App,
    math::Vec2,
    prelude::{Entity, Visibility},
    sprite::Sprite,
    transform::components::Transform,
};
use gdtf_battle_presenter::{
    CharacterRoles, DrawnLife, DrawnPose, DrawnPosition, GangerSprite, GangerSprites, facing_frame,
};
use gdtf_battle_sim::{
    act_log::{PoseFacts, SuppressedNow},
    ganger::{Aiming, Facing},
    prelude::{CellLevel, Direction, Faction, LifeState, Position, Stance, StanceKind},
};

/// The resolved `CharacterRoles` resource as a clone, or `None` if absent.
pub(crate) fn character_roles(app: &App) -> Option<CharacterRoles> {
    app.world().get_resource::<CharacterRoles>().cloned()
}

/// All `GangerSprite` markers in the world, paired with the sim entity each mirrors and
/// its sprite's atlas index / custom-size / translation. (Visibility is asserted per-sim
/// via [`visibility_of_sim`], so it is not snapshotted here.)
pub(crate) struct DrawnGanger {
    /// The presenter sprite entity.
    pub(crate) sprite_entity: Entity,
    /// The sim ganger entity it mirrors.
    pub(crate) sim_entity:    Entity,
    /// The sprite's texture-atlas index (if it carries an atlas).
    pub(crate) atlas_index:   Option<usize>,
    /// The sprite's `custom_size`.
    pub(crate) custom_size:   Option<Vec2>,
    /// The sprite's world translation.
    pub(crate) translation:   bevy::math::Vec3,
}

/// Snapshot every drawn ganger sprite (marker + sprite + transform).
pub(crate) fn drawn_gangers(app: &mut App) -> Vec<DrawnGanger> {
    let mut q = app
        .world_mut()
        .query::<(Entity, &GangerSprite, &Sprite, &Transform)>();
    q.iter(app.world())
        .map(|(sprite_entity, marker, sprite, transform)| DrawnGanger {
            sprite_entity,
            sim_entity: marker.entity,
            atlas_index: sprite.texture_atlas.as_ref().map(|a| a.index),
            custom_size: sprite.custom_size,
            translation: transform.translation,
        })
        .collect()
}

/// The sim entity that occupies `at`, found via its `Position` (the setup spawns one
/// ganger per authored cell). `None` if no ganger is there.
pub(crate) fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

/// The expected atlas index for `faction` facing `facing`, read STRUCTURALLY from the
/// data table + the 8->4 map — never a literal.
pub(crate) fn expected_index(roles: &CharacterRoles, faction: u8, facing: Direction) -> usize {
    *roles.base_for(Faction::new(faction)) + *facing_frame(facing)
}

/// The linear-RGB luminance proxy (unweighted channel sum) of a sprite color — enough to
/// assert the dim/brighten DIRECTION of the stance/aiming re-tint without pinning exact
/// channel values. `None` colors (no sprite) sort to `0.0`.
pub(crate) fn luminance(color: Option<bevy::color::Color>) -> f32 {
    match color {
        Some(c) => {
            let lin = c.to_linear();
            lin.red + lin.green + lin.blue
        }
        None => 0.0,
    }
}

/// The `Sprite.color` of the presenter sprite `entity`.
pub(crate) fn sprite_color(app: &mut App, entity: Entity) -> Option<bevy::color::Color> {
    let mut q = app.world_mut().query::<&Sprite>();
    q.get(app.world(), entity).ok().map(|s| s.color)
}

/// The presenter sprite's atlas index for sim ganger `sim` (looked up through
/// `GangerSprites`), or `None` while unmapped / not yet materialized.
pub(crate) fn atlas_index_of_sim(app: &mut App, sim: Entity) -> Option<usize> {
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Sprite>();
    q.get(app.world(), sprite)
        .ok()
        .and_then(|s| s.texture_atlas.as_ref().map(|a| a.index))
}

/// Whether two sprite colors are the SAME colour within tolerance, compared on their linear
/// RGB channels — so two `Color`s carrying the same colour compare equal despite a possibly
/// different enum variant (the appearance classifier composes through the linear pipeline).
/// `None` colors never match.
pub(crate) fn same_color(a: Option<bevy::color::Color>, b: Option<bevy::color::Color>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            let (la, lb) = (a.to_linear(), b.to_linear());
            (la.red - lb.red).abs() < 1.0e-4
                && (la.green - lb.green).abs() < 1.0e-4
                && (la.blue - lb.blue).abs() < 1.0e-4
                && (la.alpha - lb.alpha).abs() < 1.0e-4
        }
        _ => false,
    }
}

/// The saturation of a sprite color — the spread between its brightest and dimmest linear
/// channel, normalized by the brightest, so a fully-grey swatch is `0.0` and a saturated one
/// approaches `1.0`. Enough to assert the DESATURATION direction of the suppressed re-tint
/// without pinning exact channel values. `None` colors (no sprite) sort to `0.0`.
pub(crate) fn saturation(color: Option<bevy::color::Color>) -> f32 {
    match color {
        Some(c) => {
            let lin = c.to_linear();
            let max = lin.red.max(lin.green).max(lin.blue);
            let min = lin.red.min(lin.green).min(lin.blue);
            if max <= f32::EPSILON {
                0.0
            } else {
                (max - min) / max
            }
        }
        None => 0.0,
    }
}

/// The world translation of the presenter sprite `entity`, if it exists.
pub(crate) fn sprite_translation(app: &mut App, entity: Entity) -> Option<bevy::math::Vec3> {
    let mut q = app.world_mut().query::<&Transform>();
    q.get(app.world(), entity).ok().map(|t| t.translation)
}

/// Sim ganger `sim`'s current drawn-pose mirror ([`DrawnPose`]), if it has been seeded.
fn drawn_pose(app: &App, sim: Entity) -> Option<DrawnPose> {
    app.world().entity(sim).get::<DrawnPose>().copied()
}

/// Overwrite the ganger's [`DrawnPose`] mirror with a new `facing`, keeping the other pose
/// fields. GTW-727 C17: the appearance resolver reads the DRAWN pose (what the playback
/// cursor has shown), so a focused presenter test drives that mirror rather than the live
/// sim component — the same role the cursor's `advance_playback` write plays in a real battle.
pub(crate) fn set_drawn_facing(app: &mut App, sim: Entity, facing: Direction) {
    if let Some(p) = drawn_pose(app, sim) {
        app.world_mut()
            .entity_mut(sim)
            .insert(DrawnPose::new(PoseFacts::new(
                Facing::new(facing),
                p.stance(),
                p.aiming(),
                SuppressedNow::new(p.suppressed()),
            )));
    }
}

/// Overwrite the ganger's [`DrawnPose`] mirror with a new `stance`, keeping the other fields
/// (GTW-727 C17 — see [`set_drawn_facing`]).
pub(crate) fn set_drawn_stance(app: &mut App, sim: Entity, stance: StanceKind) {
    if let Some(p) = drawn_pose(app, sim) {
        app.world_mut()
            .entity_mut(sim)
            .insert(DrawnPose::new(PoseFacts::new(
                p.facing(),
                Stance::new(stance),
                p.aiming(),
                SuppressedNow::new(p.suppressed()),
            )));
    }
}

/// Overwrite the ganger's [`DrawnPose`] mirror with a new `aiming` flag, keeping the other
/// fields (GTW-727 C17 — see [`set_drawn_facing`]).
pub(crate) fn set_drawn_aiming(app: &mut App, sim: Entity, aiming: bool) {
    if let Some(p) = drawn_pose(app, sim) {
        app.world_mut()
            .entity_mut(sim)
            .insert(DrawnPose::new(PoseFacts::new(
                p.facing(),
                p.stance(),
                Aiming::new(aiming),
                SuppressedNow::new(p.suppressed()),
            )));
    }
}

/// Overwrite the ganger's [`DrawnPose`] mirror with a new `suppressed` flag, keeping the
/// other fields. GTW-727 C17 made suppression a `DrawnPose` FIELD (the
/// `RemovedComponents<Suppressed>` drain is gone), so both an apply and a clear are ordinary
/// pose value changes the resolver observes.
pub(crate) fn set_drawn_suppressed(app: &mut App, sim: Entity, suppressed: bool) {
    if let Some(p) = drawn_pose(app, sim) {
        app.world_mut()
            .entity_mut(sim)
            .insert(DrawnPose::new(PoseFacts::new(
                p.facing(),
                p.stance(),
                p.aiming(),
                SuppressedNow::new(suppressed),
            )));
    }
}

/// Overwrite the ganger's [`DrawnLife`] mirror — the cursor-time life state the appearance
/// resolver and the death despawn now read (GTW-727 C17 — see [`set_drawn_facing`]).
pub(crate) fn set_drawn_life(app: &mut App, sim: Entity, life: LifeState) {
    app.world_mut().entity_mut(sim).insert(DrawnLife::new(life));
}

/// Overwrite the ganger's [`DrawnPosition`] mirror — the cell the sprite mover now glides to
/// (GTW-727 C17 — see [`set_drawn_facing`]).
pub(crate) fn set_drawn_position(app: &mut App, sim: Entity, at: CellLevel) {
    app.world_mut()
        .entity_mut(sim)
        .insert(DrawnPosition::seeded(Position::new(at)));
}

/// The visibility of the presenter sprite mirroring sim ganger `sim` (via the map).
pub(crate) fn visibility_of_sim(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Visibility>();
    q.get(app.world(), sprite).ok().copied()
}
