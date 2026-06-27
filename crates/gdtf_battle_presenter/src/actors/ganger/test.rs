//! Unit tests for the ganger draw: the facing map, the atlas-index sum, the role table,
//! the per-ganger tints, and the character-roles hot-reload re-index.

use bevy::{ecs::system::RunSystemOnce, image::TextureAtlas, prelude::*};
use gdtf_battle_sim::{Direction, Facing, Faction, LifeState};

use super::{
    frame::{FacingFrame, atlas_index, facing_frame},
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    systems::reindex_ganger_sprites_on_character_roles_change,
    tint::{faction_tint, ganger_tint},
};
use crate::TileIndex;

/// AC2 — the 8->4 facing map maps each of the 8 directions to the documented frame,
/// exhaustively. `North/NorthEast/NorthWest -> UP`, `East -> RIGHT`,
/// `SouthEast/South/SouthWest -> DOWN`, `West -> LEFT`.
#[test]
fn facing_frame_maps_all_eight_directions() {
    assert_eq!(facing_frame(Direction::North), FacingFrame::UP, "N -> UP");
    assert_eq!(
        facing_frame(Direction::NorthEast),
        FacingFrame::UP,
        "NE -> UP",
    );
    assert_eq!(
        facing_frame(Direction::NorthWest),
        FacingFrame::UP,
        "NW -> UP",
    );
    assert_eq!(
        facing_frame(Direction::East),
        FacingFrame::RIGHT,
        "E -> RIGHT",
    );
    assert_eq!(
        facing_frame(Direction::SouthEast),
        FacingFrame::DOWN,
        "SE -> DOWN",
    );
    assert_eq!(
        facing_frame(Direction::South),
        FacingFrame::DOWN,
        "S -> DOWN",
    );
    assert_eq!(
        facing_frame(Direction::SouthWest),
        FacingFrame::DOWN,
        "SW -> DOWN",
    );
    assert_eq!(
        facing_frame(Direction::West),
        FacingFrame::LEFT,
        "W -> LEFT"
    );
}

/// The four frame offsets are the documented column offsets `0..=3` in the sheet's
/// per-actor order (LEFT/DOWN/UP/RIGHT) — the structural pins the atlas-index sum
/// relies on.
#[test]
fn facing_frame_offsets_are_zero_to_three() {
    assert_eq!(*FacingFrame::LEFT, 0, "LEFT is col+0");
    assert_eq!(*FacingFrame::DOWN, 1, "DOWN is col+1");
    assert_eq!(*FacingFrame::UP, 2, "UP is col+2");
    assert_eq!(*FacingFrame::RIGHT, 3, "RIGHT is col+3");
}

/// The atlas index is `faction_base + facing_frame`, read structurally from the
/// table + the map — never a literal. Built from an arbitrary in-test table so it
/// pins the SUM mechanism, not the shipped data.
#[test]
fn atlas_index_is_base_plus_frame() {
    let roles = CharacterRoles {
        faction_0: TileIndex::new(10),
        faction_1: TileIndex::new(20),
    };
    // Faction 0 facing East -> base 10 + RIGHT (3) = 13.
    assert_eq!(
        atlas_index(&roles, Faction::new(0), Facing::new(Direction::East)),
        13,
        "faction 0 + East = faction_0 base + RIGHT offset",
    );
    // Faction 1 facing North -> base 20 + UP (2) = 22.
    assert_eq!(
        atlas_index(&roles, Faction::new(1), Facing::new(Direction::North)),
        22,
        "faction 1 + North = faction_1 base + UP offset",
    );
}

/// `base_for` resolves each faction to its authored base, and an out-of-table
/// faction falls back to faction 0 (no panic).
#[test]
fn base_for_resolves_factions_and_falls_back() {
    let roles = CharacterRoles {
        faction_0: TileIndex::new(0),
        faction_1: TileIndex::new(4),
    };
    assert_eq!(roles.base_for(Faction::new(0)), TileIndex::new(0));
    assert_eq!(roles.base_for(Faction::new(1)), TileIndex::new(4));
    // An out-of-table gang index (none exist in the two-gang design) -> faction 0.
    assert_eq!(roles.base_for(Faction::new(7)), TileIndex::new(0));
}

/// The two factions resolve to two DISTINCT base indices in the shipped table — the
/// visibly-distinct-actors guarantee, asserted structurally (not a magnitude pin).
#[test]
fn shipped_character_roles_ron_parses_with_distinct_factions() {
    const SHIPPED: &str =
        include_str!("../../../../../assets/sprites/character_roles.spritedef.ron");
    let parsed: Result<CharacterRoles, _> = ron::de::from_str(SHIPPED);
    assert!(
        parsed.is_ok(),
        "shipped character_roles.ron must parse into CharacterRoles, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(roles) = parsed else {
        return;
    };
    assert_ne!(
        roles.faction_0, roles.faction_1,
        "the two factions must map to two visibly distinct actor base tiles",
    );
}

/// The Downed tint differs from both factions' live tints — the documented Downed
/// delta is a visible re-tint.
#[test]
fn downed_tint_differs_from_live_faction_tints() {
    assert_ne!(
        ganger_tint(Faction::new(0), LifeState::Downed),
        ganger_tint(Faction::new(0), LifeState::Alive),
        "a Downed faction-0 ganger tints differently from a live one",
    );
    assert_ne!(
        ganger_tint(Faction::new(1), LifeState::Downed),
        ganger_tint(Faction::new(1), LifeState::Alive),
        "a Downed faction-1 ganger tints differently from a live one",
    );
}

/// The two factions draw two distinct live tints — the "faction-coloured" signal.
#[test]
fn faction_tints_are_distinct() {
    assert_ne!(
        faction_tint(Faction::new(0)),
        faction_tint(Faction::new(1)),
        "the two factions must read as two distinct colours",
    );
}

/// C10(c): the ganger re-index re-indexes a MAPPED ganger sprite's atlas index to the new
/// `CharacterRoles` base when the table hot-reloads — driven through the REAL registered
/// system (`reindex_ganger_sprites_on_character_roles_change`) via `run_system_once`.
///
/// A presenter sprite carrying a `TextureAtlas` (index seeded at the OLD base) is mapped to
/// a sim ganger (`Faction` 0, facing East). After overwriting `CharacterRoles` with a new
/// faction-0 base and running the system, the sprite's atlas index is the NEW
/// `atlas_index(&new_roles, faction_0, East)` — proving the re-index reads the CURRENT sim
/// state against the freshly-reloaded table.
///
/// Pin-discriminating: dropping the re-index leaves the sprite on the OLD base+frame index;
/// re-tinting instead of re-indexing would not move the atlas index.
#[test]
fn character_roles_change_reindexes_mapped_ganger_sprite() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Seed a starting CharacterRoles (faction-0 base 10) and a matching sprite index.
    let old_roles = CharacterRoles {
        faction_0: TileIndex::new(10),
        faction_1: TileIndex::new(40),
    };
    let faction = Faction::new(0);
    let facing = Facing::new(Direction::East);
    let old_index = atlas_index(&old_roles, faction, facing);

    // The PRESENTER sprite: a `Sprite` carrying a `TextureAtlas` at the OLD index, tagged
    // `GangerSprite`. Its layout handle is a throwaway default (the re-index only mutates
    // `texture_atlas.index`, never the layout). A recognizable tint lets the "re-index ONLY,
    // never re-tint" clause be checked afterward.
    let sim_entity_placeholder = app.world_mut().spawn_empty().id();
    let original_color = Color::srgb(0.25, 0.5, 0.75);
    let sprite = Sprite {
        texture_atlas: Some(TextureAtlas {
            layout: Handle::default(),
            index:  old_index,
        }),
        color: original_color,
        ..Sprite::default()
    };
    let presenter = app
        .world_mut()
        .spawn((
            sprite,
            GangerSprite {
                entity: sim_entity_placeholder,
            },
        ))
        .id();

    // The SIM ganger: faction + facing only (the fields the re-index reads).
    let sim = app.world_mut().spawn((faction, facing)).id();

    // Map sim -> presenter, and install the OLD roles.
    let mut sprites = GangerSprites::default();
    sprites.insert(sim, presenter);
    app.world_mut().insert_resource(sprites);
    app.world_mut().insert_resource(old_roles);

    // Hot-reload: overwrite CharacterRoles with a NEW, distinct faction-0 base.
    let new_roles = CharacterRoles {
        faction_0: TileIndex::new(120),
        faction_1: TileIndex::new(40),
    };
    let new_index = atlas_index(&new_roles, faction, facing);
    assert_ne!(
        new_index, old_index,
        "precondition: the new base must move the index"
    );
    app.world_mut().insert_resource(new_roles);

    // Run the REAL registered re-index system once.
    let result = app
        .world_mut()
        .run_system_once(reindex_ganger_sprites_on_character_roles_change);
    assert!(result.is_ok(), "the re-index system must run cleanly");

    let sprite = app
        .world()
        .entity(presenter)
        .get::<Sprite>()
        .and_then(|s| s.texture_atlas.as_ref().map(|a| a.index));
    assert_eq!(
        sprite,
        Some(new_index),
        "a CharacterRoles hot-reload must re-index the mapped ganger sprite to the new base",
    );

    // The re-index must NOT touch the tint (the life/stance/aiming tint is owned elsewhere).
    assert_eq!(
        app.world()
            .entity(presenter)
            .get::<Sprite>()
            .map(|s| s.color),
        Some(original_color),
        "the re-index must preserve the existing tint (re-index ONLY)",
    );
}
