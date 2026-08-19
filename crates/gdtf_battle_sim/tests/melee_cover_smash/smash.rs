use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{MeleeRequested, melee_tu_cost},
    entity::TerrainPieceKind,
    ganger::Direction,
    prelude::Tu,
    test_support::SituationBuilder,
    weapon::{FightMode, MeleeWeapon, Wields},
};

use super::harness::*;

/// What one strike with the melee weapon this attacker holds charges, off the live world.
fn strike_cost(app: &App, attacker: Entity) -> Option<Tu> {
    let world = app.world();
    let weapon = world
        .get::<Wields>(attacker)?
        .melee_weapon(|entity| world.get::<MeleeWeapon>(entity).is_some())?;
    world.get::<FightMode>(weapon).map(melee_tu_cost)
}

// === C5(a) — a smash on an adjacent Cover cell REDUCES its HP, spends TU, emits MeleeResolved. ===

#[test]
fn smash_reduces_adjacent_cover_hp_and_emits_resolved() {
    let (mut app, seed) = battle_app(0x5508_0A0A);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1_000, TerrainPieceKind::Cover);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let (Some(tu_before), Some(hp_before)) = (tu_of(&app, attacker_entity), cover_hp(&app, cover))
    else {
        unreachable!("the attacker carries Tu and the cover cell was seeded");
    };

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, cover));
    step(&mut app, 3);

    let Some(hp_after) = cover_hp(&app, cover) else {
        unreachable!("the cover entry persists");
    };
    assert!(
        hp_after < hp_before,
        "C5(a): a melee cover-smash REDUCES the cell's structure HP ({hp_after} < {hp_before})",
    );

    assert!(
        tu_of(&app, attacker_entity).is_some_and(|tu| tu < tu_before),
        "C5(a): the attacker's TU is spent by the smash",
    );

    assert!(
        melee_hits(&app) >= 1,
        "C5(a): a melee cover-smash emits MeleeResolved (the FX signal) — wired end-to-end",
    );
}

#[test]
fn repeated_smashing_destroys_cover_and_fires_the_destroyed_signal() {
    let (mut app, seed) = battle_app(0x5508_0B0B);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1, TerrainPieceKind::Cover);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };

    for _ in 0..3 {
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker_entity, cover));
        step(&mut app, 3);
    }

    assert!(
        cover_destroyed_flag(&app, cover),
        "C5(b): sufficient melee smashing DESTROYS the cover (its ledger HP reached zero)",
    );
    assert_eq!(
        cover_hp(&app, cover),
        Some(0),
        "C5(b): a destroyed cover's current HP is zero",
    );

    assert!(
        destroyed_hits(&app) >= 1,
        "C5(b): destroying the cover fires the TerrainPieceDestroyed signal (the FX bridge)",
    );
    assert!(
        melee_hits(&app) >= 1,
        "C5(b): the smash emits MeleeResolved",
    );
}

#[test]
fn a_smashed_piece_reports_the_kind_the_ledger_held() {
    let (mut app, seed) = battle_app(0x5508_0F0F);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let wall = ground(6, 5);
    let cover = ground(5, 6);
    seed_cover(&mut app, wall, 1, TerrainPieceKind::Wall);
    seed_cover(&mut app, cover, 1, TerrainPieceKind::Cover);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };

    for at in [wall, cover] {
        for _ in 0..8 {
            if cover_destroyed_flag(&app, at) {
                break;
            }
            if let Some(mut pool) = app.world_mut().get_mut::<Tu>(attacker_entity) {
                *pool = Tu::new(250);
            }
            app.world_mut()
                .write_message(MeleeRequested::new_structural(attacker_entity, at));
            step(&mut app, 3);
        }
    }

    let kinds: Vec<TerrainPieceKind> = destroyed_messages(&app)
        .into_iter()
        .map(|message| message.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![TerrainPieceKind::Wall, TerrainPieceKind::Cover],
        "a melee smash reports the kind the ledger entry held — a smashed Wall then a smashed \
         Cover, found {kinds:?}",
    );
}

#[test]
fn smashing_an_unseeded_cell_mints_a_cover_kind_entry() {
    let (mut app, seed) = battle_app(0x5508_0E0E);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let bare = ground(6, 5);
    assert!(
        cover_entry_at(&app, bare).is_none(),
        "precondition: the target cell was never seeded, so the smash must fall back",
    );

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, bare));
    step(&mut app, 3);

    let minted = cover_entry_at(&app, bare);
    assert!(
        minted.is_some(),
        "smashing an unseeded cell must mint a ledger entry from the melee fallback",
    );
    let Some(minted) = minted else { return };
    assert_eq!(
        minted.kind,
        TerrainPieceKind::Cover,
        "the melee structure fallback stands in as COVER, so the minted entry's kind is Cover, \
         found {:?}",
        minted.kind,
    );
}

#[test]
fn the_structural_smash_is_seed_independent_no_fight_roll() {
    let run_with_seed = |seed: u64| -> Option<u32> {
        let (mut app, seed) = battle_app(seed);
        with_logs(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let cover = ground(6, 5);
        seed_cover(&mut app, cover, 1_000, TerrainPieceKind::Cover);
        let Some(attacker_entity) = player_ganger(&mut app) else {
            unreachable!("setup spawns one player attacker");
        };
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker_entity, cover));
        step(&mut app, 3);
        cover_hp(&app, cover)
    };

    let a = run_with_seed(0x0000_1111);
    let b = run_with_seed(0xFFFF_EEEE);
    assert_eq!(
        a, b,
        "C5(c): the structural smash is SEED-INDEPENDENT (no opposed roll, no FightRng draw) — \
         two different seeds leave the same cover HP: {a:?} vs {b:?}",
    );
    assert!(
        a.is_some_and(|hp| hp < 1_000),
        "precondition: the smash actually reduced the cover HP (a real, non-vacuous outcome)",
    );
}

#[test]
fn a_smash_the_pool_cannot_cover_leaves_the_cover_and_the_pool_alone() {
    let (mut app, seed) = battle_app(0x5508_0D0D);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1_000, TerrainPieceKind::Cover);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let (Some(cost), Some(hp_before)) = (strike_cost(&app, attacker_entity), cover_hp(&app, cover))
    else {
        unreachable!("the attacker holds a melee weapon and the cover cell was seeded");
    };
    assert!(
        *cost > 0,
        "the test melee weapon must charge something, or a below-cost pool proves nothing",
    );
    if let Some(mut pool) = app.world_mut().get_mut::<Tu>(attacker_entity) {
        *pool = Tu::new(*cost - 1);
    }

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, cover));
    step(&mut app, 3);

    assert_eq!(
        tu_of(&app, attacker_entity),
        Some(*cost - 1),
        "an attacker that cannot cover the smash spends nothing",
    );
    assert_eq!(
        cover_hp(&app, cover),
        Some(hp_before),
        "a smash the attacker cannot afford leaves the cover's HP untouched",
    );
    assert!(
        !cover_destroyed_flag(&app, cover),
        "a smash the attacker cannot afford destroys nothing",
    );
    assert_eq!(
        destroyed_hits(&app),
        0,
        "a smash the attacker cannot afford fires NO TerrainPieceDestroyed",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "a smash the attacker cannot afford emits NO MeleeResolved",
    );

    if let Some(mut pool) = app.world_mut().get_mut::<Tu>(attacker_entity) {
        *pool = Tu::new(*cost);
    }
    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, cover));
    step(&mut app, 3);
    assert_eq!(
        tu_of(&app, attacker_entity),
        Some(0),
        "the same smash from a pool that covers the cost is charged for",
    );
    assert!(
        cover_hp(&app, cover).is_some_and(|hp| hp < hp_before),
        "the same smash from a pool that covers the cost does reduce the cover's HP",
    );
}

#[test]
fn a_non_adjacent_structure_is_not_smashed() {
    let (mut app, seed) = battle_app(0x5508_0C0C);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let far_cover = ground(8, 5);
    seed_cover(&mut app, far_cover, 1_000, TerrainPieceKind::Cover);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let Some(hp_before) = cover_hp(&app, far_cover) else {
        unreachable!("the far cover cell was seeded");
    };

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, far_cover));
    step(&mut app, 3);

    assert_eq!(
        cover_hp(&app, far_cover),
        Some(hp_before),
        "the 8-adjacency gate held — a non-adjacent structure takes NO melee damage",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "a non-adjacent structure smash emits NO MeleeResolved",
    );
}
