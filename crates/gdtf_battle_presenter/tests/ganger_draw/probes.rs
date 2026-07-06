//! Cross-surface sprite readback probes shared by the `ganger_draw` concern modules.

use bevy::{
    app::App,
    math::Vec2,
    prelude::{Entity, Visibility},
    sprite::Sprite,
    transform::components::Transform,
};
use gdtf_battle_presenter::{CharacterRoles, GangerSprite, GangerSprites, facing_frame};
use gdtf_battle_sim::prelude::{CellLevel, Direction, Faction, Position};

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

/// The world translation of the presenter sprite `entity`, if it exists.
pub(crate) fn sprite_translation(app: &mut App, entity: Entity) -> Option<bevy::math::Vec3> {
    let mut q = app.world_mut().query::<&Transform>();
    q.get(app.world(), entity).ok().map(|t| t.translation)
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
