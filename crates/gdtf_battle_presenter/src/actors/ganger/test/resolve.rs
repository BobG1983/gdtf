//! The GTW-631 ONE-writer system (`resolve_ganger_appearance`): change-driven stamping
//! of BOTH `Sprite` channels, the `CharacterRoles`-change full restamp (the dissolved
//! re-index — now healing a stale tint too), and the suppression re-tint.
//!
//! GTW-727 C17 re-sourced the resolver onto the presenter's `DrawnPose` / `DrawnLife`
//! mirrors (what the playback cursor has SHOWN), so these tests drive those mirrors rather
//! than the live sim components. The suppression clear that used to ride a
//! `RemovedComponents<Suppressed>` drain is now an ordinary `DrawnPose` field change.

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

/// The harness ganger's canonical drawn pose with `suppressed` toggled — East / Standing /
/// not-aiming, the values `writer_app` seeds and the suppression test flips.
fn pose(suppressed: bool) -> DrawnPose {
    DrawnPose::new(PoseFacts::new(
        Facing::new(Direction::East),
        Stance::new(StanceKind::Standing),
        Aiming::new(false),
        SuppressedNow::new(suppressed),
    ))
}

/// A minimal app running ONLY the real writer, with a mapped sim ganger + presenter
/// sprite pair — returns `(app, sim, presenter)`. The ganger carries the presenter's
/// `DrawnPose` / `DrawnLife` mirrors (the resolver's GTW-727 C17 inputs), and the sprite is
/// seeded DELIBERATELY off-canonical (a poison index + tint) so the first stamp is observable.
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

/// A recognizable OFF-canonical tint no classifier verdict produces.
fn poison() -> Color {
    Color::srgb(0.123, 0.456, 0.789)
}

/// The presenter sprite's stamped `(atlas index, tint)` pair.
fn stamped(app: &App, presenter: Entity) -> (Option<usize>, Option<Color>) {
    let sprite = app.world().entity(presenter).get::<Sprite>();
    (
        sprite.and_then(|s| s.texture_atlas.as_ref().map(|a| a.index)),
        sprite.map(|s| s.color),
    )
}

/// The classifier verdict for the harness ganger's CURRENT DRAWN state (the same
/// `DrawnPose` / `DrawnLife` inputs the resolver reads, GTW-727 C17).
fn expected(app: &mut App, sim: Entity) -> GangerAppearance {
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &DrawnPose, &DrawnLife)>();
    let Ok((faction, pose, life)) = query.get(world, sim) else {
        // Unreachable in these tests; a missing ganger fails the caller's assert_eq.
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

/// The writer stamps BOTH channels from the classifier, is CHANGE-DRIVEN (a quiet frame
/// re-stamps nothing), and a `CharacterRoles` overwrite (the hot-reload signal) restamps
/// every mapped ganger — index moved to the new base AND a stale tint healed. The old
/// re-index's "preserve the tint" convention is gone: one writer, one derivation, so an
/// off-canonical colour cannot survive a restamp (GTW-631 A1).
#[test]
fn roles_change_restamps_both_channels_and_quiet_frames_stamp_nothing() {
    let (mut app, sim, presenter) = writer_app();

    // Frame 1: the initial-resolve stamp (roles read as changed on the first run —
    // documented harmless) replaces the poison seed with the classifier verdict.
    app.update();
    let verdict = expected(&mut app, sim);
    assert_eq!(
        stamped(&app, presenter),
        (Some(verdict.atlas_index), Some(verdict.tint)),
        "the first stamp writes the classifier's verdict onto BOTH channels",
    );

    // Re-poison the sprite directly, then run a QUIET frame: no drawn-state change, no roles
    // change — the writer is change-driven, so the poison survives.
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

    // Overwrite CharacterRoles (the hot-reload redrive marks it changed): EVERY mapped
    // ganger restamps — the index moves to the new base and the poison tint heals.
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

/// `Changed<DrawnLife>` rides the ONE writer: the cursor showing the ganger DOWNED re-tints
/// it to the classifier's Downed verdict on the same sprite, atlas index untouched.
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

/// Suppression rides the ONE writer end to end through the `DrawnPose` field (GTW-727 C17):
/// showing a suppressed pose (a `Changed<DrawnPose>`) drains the tint, and showing an
/// un-suppressed pose again restores the baseline. The old `RemovedComponents<Suppressed>`
/// drain is gone — a clear is now just another field value change the resolver observes.
#[test]
fn suppression_apply_and_removal_restamp_through_the_one_writer() {
    let (mut app, sim, presenter) = writer_app();
    app.update();
    let (_, baseline) = stamped(&app, presenter);

    // The cursor shows a suppressed pose — an ordinary DrawnPose value change.
    app.world_mut().entity_mut(sim).insert(pose(true));
    app.update();
    let (_, drained) = stamped(&app, presenter);
    assert_ne!(
        drained, baseline,
        "applying suppression re-tints the sprite"
    );

    // The cursor shows the un-suppressed pose again — the drain-free clear the deleted
    // RemovedComponents path used to handle.
    app.world_mut().entity_mut(sim).insert(pose(false));
    app.update();
    let (_, restored) = stamped(&app, presenter);
    assert_eq!(
        restored, baseline,
        "clearing suppression (a DrawnPose field change) restores the baseline tint",
    );
}
