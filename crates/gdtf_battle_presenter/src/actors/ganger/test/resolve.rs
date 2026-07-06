//! The GTW-631 ONE-writer system (`resolve_ganger_appearance`): change-driven stamping
//! of BOTH `Sprite` channels, the `CharacterRoles`-change full restamp (the dissolved
//! re-index — now healing a stale tint too), and the suppression-removal drain.

use bevy::{image::TextureAtlas, prelude::*};
use gdtf_battle_sim::{
    Aiming, Cell, CellLevel, Direction, Facing, Faction, Level, LifeState, Stance, StanceKind,
    Suppressed, SuppressorCell,
};

use super::super::{
    appearance::{GangerAppearance, ganger_sprite_appearance, resolve_ganger_appearance},
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
};
use crate::TileIndex;

/// A minimal app running ONLY the real writer, with a mapped sim ganger + presenter
/// sprite pair — returns `(app, sim, presenter)`. The sprite is seeded DELIBERATELY
/// off-canonical (a poison index + tint) so the first stamp is observable.
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
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            LifeState::Alive,
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

/// The classifier verdict for the harness ganger's CURRENT sim state.
fn expected(app: &mut App, sim: Entity) -> GangerAppearance {
    let world = app.world_mut();
    let mut query = world.query::<(
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        &LifeState,
        Option<&Suppressed>,
    )>();
    let Ok((faction, facing, stance, aiming, life, suppressed)) = query.get(world, sim) else {
        // Unreachable in these tests; a missing ganger fails the caller's assert_eq.
        return GangerAppearance {
            atlas_index: usize::MAX,
            tint:        Color::NONE,
        };
    };
    let (faction, facing, stance, aiming, life, suppressed) = (
        *faction,
        *facing,
        *stance,
        *aiming,
        *life,
        suppressed.is_some(),
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

    // Re-poison the sprite directly, then run a QUIET frame: no sim change, no roles
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

/// `Changed<LifeState>` rides the ONE writer: downing the ganger re-tints it to the
/// classifier's Downed verdict on the same sprite, atlas index untouched.
#[test]
fn life_state_change_retints_through_the_one_writer() {
    let (mut app, sim, presenter) = writer_app();
    app.update();
    let (alive_index, alive_tint) = stamped(&app, presenter);

    app.world_mut().entity_mut(sim).insert(LifeState::Downed);
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

/// Suppression rides the ONE writer end to end: inserting `Suppressed` (a `Changed`)
/// drains the tint, and REMOVING it (a removal `Changed` cannot observe) restores the
/// baseline through the `RemovedComponents<Suppressed>` drain.
#[test]
fn suppression_apply_and_removal_restamp_through_the_one_writer() {
    let (mut app, sim, presenter) = writer_app();
    app.update();
    let (_, baseline) = stamped(&app, presenter);

    let from = SuppressorCell::new(CellLevel::new(Cell::new(9, 6), Level::new(0)));
    app.world_mut()
        .entity_mut(sim)
        .insert(Suppressed::new(from));
    app.update();
    let (_, drained) = stamped(&app, presenter);
    assert_ne!(
        drained, baseline,
        "applying suppression re-tints the sprite"
    );

    app.world_mut().entity_mut(sim).remove::<Suppressed>();
    app.update();
    let (_, restored) = stamped(&app, presenter);
    assert_eq!(
        restored, baseline,
        "clearing suppression (a REMOVAL) restores the baseline tint via the drain",
    );
}
