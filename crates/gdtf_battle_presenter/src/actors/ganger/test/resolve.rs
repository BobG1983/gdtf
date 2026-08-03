use bevy::{image::TextureAtlas, prelude::*};
use gdtf_battle_sim::{
    act_log::{PoseFacts, SuppressedNow},
    ganger::{Aiming, Facing},
    prelude::{Direction, Faction, LifeState, Stance, StanceKind},
};

use super::super::{
    appearance::{GangerAppearance, ganger_sprite_appearance, resolve_ganger_appearance},
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
};
use crate::{
    TileIndex,
    playback::{DrawnLife, DrawnPose},
};

fn pose(suppressed: bool) -> DrawnPose {
    DrawnPose::new(PoseFacts::new(
        Facing::new(Direction::East),
        Stance::new(StanceKind::Standing),
        Aiming::new(false),
        SuppressedNow::new(suppressed),
    ))
}

fn writer_app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, resolve_ganger_appearance);
    app.world_mut().insert_resource(CharacterRoles {
        faction_0: TileIndex::new(10),
        faction_1: TileIndex::new(40),
    });

    let sim = app
        .world_mut()
        .spawn((
            Faction::new(0),
            pose(false),
            DrawnLife::new(LifeState::Alive),
        ))
        .id();
    let presenter = app
        .world_mut()
        .spawn((
            Sprite {
                texture_atlas: Some(TextureAtlas {
                    layout: Handle::default(),
                    index:  999,
                }),
                color: poison(),
                ..Sprite::default()
            },
            GangerSprite { entity: sim },
        ))
        .id();
    let mut sprites = GangerSprites::default();
    sprites.insert(sim, presenter);
    app.world_mut().insert_resource(sprites);
    (app, sim, presenter)
}

fn poison() -> Color {
    Color::srgb(0.123, 0.456, 0.789)
}

fn stamped(app: &App, presenter: Entity) -> (Option<usize>, Option<Color>) {
    let sprite = app.world().entity(presenter).get::<Sprite>();
    (
        sprite.and_then(|s| s.texture_atlas.as_ref().map(|a| a.index)),
        sprite.map(|s| s.color),
    )
}

fn expected(app: &mut App, sim: Entity) -> GangerAppearance {
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &DrawnPose, &DrawnLife)>();
    let Ok((faction, pose, life)) = query.get(world, sim) else {
        return GangerAppearance {
            atlas_index: usize::MAX,
            tint:        Color::NONE,
        };
    };
    let (faction, facing, stance, aiming, life, suppressed) = (
        *faction,
        pose.facing(),
        pose.stance(),
        pose.aiming(),
        **life,
        pose.suppressed(),
    );
    let roles = world.resource::<CharacterRoles>();
    ganger_sprite_appearance(faction, facing, stance, aiming, life, suppressed, roles)
}

#[test]
fn roles_change_restamps_both_channels_and_quiet_frames_stamp_nothing() {
    let (mut app, sim, presenter) = writer_app();

    app.update();
    let verdict = expected(&mut app, sim);
    assert_eq!(
        stamped(&app, presenter),
        (Some(verdict.atlas_index), Some(verdict.tint)),
        "the first stamp writes the classifier's verdict onto BOTH channels",
    );

    if let Some(mut sprite) = app.world_mut().entity_mut(presenter).get_mut::<Sprite>() {
        sprite.color = poison();
    }
    app.update();
    let (_, tint) = stamped(&app, presenter);
    assert_eq!(
        tint,
        Some(poison()),
        "a quiet frame stamps nothing (the writer is change-driven, not per-frame)",
    );

    app.world_mut().insert_resource(CharacterRoles {
        faction_0: TileIndex::new(120),
        faction_1: TileIndex::new(40),
    });
    app.update();
    let verdict = expected(&mut app, sim);
    assert_eq!(
        verdict.atlas_index, 123,
        "precondition: the new base moved the index"
    );
    assert_eq!(
        stamped(&app, presenter),
        (Some(verdict.atlas_index), Some(verdict.tint)),
        "a roles change restamps BOTH channels from the fresh table (stale tint healed)",
    );
}

#[test]
fn life_state_change_retints_through_the_one_writer() {
    let (mut app, sim, presenter) = writer_app();
    app.update();
    let (alive_index, alive_tint) = stamped(&app, presenter);

    app.world_mut()
        .entity_mut(sim)
        .insert(DrawnLife::new(LifeState::Downed));
    app.update();
    let verdict = expected(&mut app, sim);
    let (downed_index, downed_tint) = stamped(&app, presenter);
    assert_eq!(
        downed_tint,
        Some(verdict.tint),
        "a Downed flip stamps the classifier's Downed tint",
    );
    assert_ne!(downed_tint, alive_tint, "the Downed re-tint is visible");
    assert_eq!(
        downed_index, alive_index,
        "a life flip never moves the atlas index"
    );
}

#[test]
fn suppression_apply_and_removal_restamp_through_the_one_writer() {
    let (mut app, sim, presenter) = writer_app();
    app.update();
    let (_, baseline) = stamped(&app, presenter);

    app.world_mut().entity_mut(sim).insert(pose(true));
    app.update();
    let (_, drained) = stamped(&app, presenter);
    assert_ne!(
        drained, baseline,
        "applying suppression re-tints the sprite"
    );

    app.world_mut().entity_mut(sim).insert(pose(false));
    app.update();
    let (_, restored) = stamped(&app, presenter);
    assert_eq!(
        restored, baseline,
        "clearing suppression (a DrawnPose field change) restores the baseline tint",
    );
}
