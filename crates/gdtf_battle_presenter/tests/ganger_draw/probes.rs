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

pub(crate) fn character_roles(app: &App) -> Option<CharacterRoles> {
    app.world().get_resource::<CharacterRoles>().cloned()
}

pub(crate) struct DrawnGanger {
        pub(crate) sprite_entity: Entity,
        pub(crate) sim_entity:    Entity,
        pub(crate) atlas_index:   Option<usize>,
        pub(crate) custom_size:   Option<Vec2>,
        pub(crate) translation:   bevy::math::Vec3,
}

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

pub(crate) fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

pub(crate) fn expected_index(roles: &CharacterRoles, faction: u8, facing: Direction) -> usize {
    *roles.base_for(Faction::new(faction)) + *facing_frame(facing)
}

pub(crate) fn luminance(color: Option<bevy::color::Color>) -> f32 {
    match color {
        Some(c) => {
            let lin = c.to_linear();
            lin.red + lin.green + lin.blue
        }
        None => 0.0,
    }
}

pub(crate) fn sprite_color(app: &mut App, entity: Entity) -> Option<bevy::color::Color> {
    let mut q = app.world_mut().query::<&Sprite>();
    q.get(app.world(), entity).ok().map(|s| s.color)
}

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

pub(crate) fn sprite_translation(app: &mut App, entity: Entity) -> Option<bevy::math::Vec3> {
    let mut q = app.world_mut().query::<&Transform>();
    q.get(app.world(), entity).ok().map(|t| t.translation)
}

fn drawn_pose(app: &App, sim: Entity) -> Option<DrawnPose> {
    app.world().entity(sim).get::<DrawnPose>().copied()
}

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

pub(crate) fn set_drawn_life(app: &mut App, sim: Entity, life: LifeState) {
    app.world_mut().entity_mut(sim).insert(DrawnLife::new(life));
}

pub(crate) fn set_drawn_position(app: &mut App, sim: Entity, at: CellLevel) {
    app.world_mut()
        .entity_mut(sim)
        .insert(DrawnPosition::seeded(Position::new(at)));
}

pub(crate) fn visibility_of_sim(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Visibility>();
    q.get(app.world(), sprite).ok().copied()
}
