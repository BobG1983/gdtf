//! GTW-660 — the emplacement-reactor weapon divergence: the reactor's eligibility
//! gates must read the SAME weapon `dispatch_fire` will fire (mounted-first, GTW-543),
//! so a reactor MANNING an emplacement whose MOUNTED gun cannot fire is skipped in the
//! gates BEFORE the opposed-check roll — never offered (and cap-charged) on its CARRIED
//! gun's eligibility.
//!
//! The filed mechanism (found in GTW-646's build, filed as GTW-660): `reactor_weapon`
//! resolved only the carried ranged weapon while `dispatch_fire` PREFERS the mounted
//! gun — so a mounted reactor with an EMPTY mount and a LOADED carried gun was offered
//! an interrupt the dispatcher then resolved against the empty mount: `ReactionsUsed`
//! incremented with zero dispatched shots, the GTW-646 spend-without-shot class in an
//! emplacement geometry.

use bevy::{app::App, ecs::relationship::RelationshipTarget as _, prelude::Entity};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, MoveRequested},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::Direction,
    magazine::{LoadedRounds, Magazine},
    metric::CellLevel,
    terrain::{
        emplacement::{EmplacementState, MountedWeaponEntity, MountedWeaponKey},
        entity::TerrainCell,
    },
    test_support::{SituationBuilder, TEST_WEAPON_KEY, test_weapon_spec},
    weapon::{WeaponName, WeaponRegistry, Wields},
};

use super::{harness::*, support::*};

/// The registry key of the emplacement's bolted-down gun.
const MOUNTED_KEY: &str = "test-mounted-gun";

/// A `WeaponRegistry` holding BOTH the default carried test gun ([`TEST_WEAPON_KEY`])
/// and the emplacement's mounted gun ([`MOUNTED_KEY`]) — the same arbitrary spec under
/// two keys (this suite discriminates by SPEND, not by damage type).
fn two_gun_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(TEST_WEAPON_KEY.to_owned()),
            test_weapon_spec(),
        ),
        (WeaponName::new(MOUNTED_KEY.to_owned()), test_weapon_spec()),
    ])
}

/// Spawn a VACANT emplacement terrain entity at `at` (state + cell + mounted-weapon
/// key + an intact armorless `CoverLedger` entry — what `setup_battle` attaches for an
/// `Emplacement` piece; the `emplacement_act` harness idiom). Returns its `Entity`.
fn spawn_emplacement(app: &mut App, at: CellLevel) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            TerrainCell::new(at),
            EmplacementState::Vacant,
            MountedWeaponKey::new(WeaponName::new(MOUNTED_KEY.to_owned())),
        ))
        .id();
    app.world_mut().resource_mut::<CoverLedger>().insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(45),
            HeightBand::High,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
        ),
    );
    entity
}

/// Man `emplacement` with `ganger` through the REAL enter act (spawns + relates the
/// mounted weapon, GTW-543) and return the spawned mount's entity.
fn man_emplacement(app: &mut App, ganger: Entity, emplacement: Entity) -> Entity {
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(ganger, emplacement));
    step(app, 3);
    let Some(mount) = app
        .world()
        .get::<MountedWeaponEntity>(emplacement)
        .map(|m| **m)
    else {
        unreachable!("the enter act spawns + records the mounted weapon");
    };
    mount
}

/// Overwrite `weapon`'s magazine with an EMPTY one (same size / reload cost) — the
/// mounted gun that cannot fire.
fn empty_magazine(app: &mut App, weapon: Entity) {
    let Some(magazine) = app.world().get::<Magazine>(weapon).copied() else {
        unreachable!("the weapon entity carries a Magazine");
    };
    let Some(mut live) = app.world_mut().get_mut::<Magazine>(weapon) else {
        unreachable!("the weapon entity carries a Magazine");
    };
    *live = Magazine::new(LoadedRounds::new(0), magazine.size(), magazine.reload_tu());
}

/// Whether `ganger` wields a LOADED gun other than `mount` — the carried weapon the
/// divergent (pre-GTW-660) eligibility gates read. Melee weapons carry no `Magazine`,
/// so a magazine-bearing non-mount wielded entity IS the carried ranged gun.
fn carried_gun_loaded(app: &mut App, ganger: Entity, mount: Entity) -> bool {
    let world = app.world_mut();
    let wielded: Vec<Entity> = {
        let Some(wields) = world.get::<Wields>(ganger) else {
            unreachable!("the fixture ganger wields weapons at setup");
        };
        wields.iter().collect()
    };
    wielded.into_iter().any(|entity| {
        entity != mount
            && world
                .get::<Magazine>(entity)
                .is_some_and(|magazine| !*magazine.is_empty())
    })
}

// === C1 — the filed divergence geometry: a mounted reactor whose MOUNT is empty while
// its CARRIED gun looks eligible must not be offered (nor cap-charged) an interrupt the
// dispatcher will resolve against the empty mount. ===

#[test]
fn empty_mounted_gun_offer_is_skipped_without_cap_spend() {
    let mut app = battle_app(forced_reaction_tuning(1));
    // The registry must carry the mount's key BEFORE setup / enter resolve weapons.
    app.insert_resource(two_gun_registry());
    with_shot_log(&mut app);

    // The PLAYER watcher at (5,5) facing East; its emplacement 8-adjacent at (5,6) —
    // OUT of every eastward sightline (rays to the mover's row y=4 never cross y=6);
    // one tough ENEMY mover NE at (7,4), inside LOS + arc + view range at spawn.
    let watcher_cell = ground(5, 5);
    let mover_start = ground(7, 4);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watcher_cell, PLAYER, Direction::East),
            tough_mover(mover_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(reactor), Some(mover)) = (
        ganger_at(&mut app, watcher_cell),
        ganger_at(&mut app, mover_start),
    ) else {
        unreachable!("setup spawns the watcher and the mover at their fixture cells");
    };

    // Man the emplacement through the REAL enter act, then build the divergence
    // geometry: the MOUNTED gun cannot fire (emptied magazine) while the CARRIED gun
    // still looks eligible (loaded), and the pool still affords a single-mode shot.
    let emplacement = spawn_emplacement(&mut app, ground(5, 6));
    let mount = man_emplacement(&mut app, reactor, emplacement);
    empty_magazine(&mut app, mount);
    assert!(
        carried_gun_loaded(&mut app, reactor, mount),
        "fixture precondition: the carried gun is loaded (only the mount is empty)",
    );
    let cost = single_fire_cost(&mut app, reactor);
    assert!(
        tu_of(&app, reactor).is_some_and(|tu| tu >= cost),
        "fixture precondition: the pool affords a single-mode interrupt after the \
         enter cost, else affordability (not the magazine) would gate the offer",
    );

    // The act: the mover steps east through the reactor's sightline.
    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 4)));
    step(&mut app, 16);

    // The mount is empty — nothing can dispatch...
    assert_eq!(
        shots_by(&app, reactor),
        0,
        "the empty mounted gun dispatches no interrupt shot",
    );
    // ...so nothing may be SPENT: the eligibility gates must read the MOUNTED gun (the
    // weapon dispatch_fire will fire), never the loaded carried gun.
    assert_eq!(
        used_of(&app, reactor),
        Some(0),
        "GTW-660: a mounted reactor whose MOUNT cannot fire is skipped before the \
         roll — never cap-charged on its CARRIED gun's eligibility",
    );
}

// === C3 — the GTW-646 spend-integrity invariant extended with the mounted geometry:
// a mixed tick of one mounted-empty reactor and one carried-eligible reactor — only
// the latter fires and spends, and Σ used == Σ dispatched holds across both. ===

#[test]
fn mixed_tick_mounted_empty_and_carried_eligible_spend_tracks_shots() {
    let mut app = battle_app(forced_reaction_tuning(1));
    app.insert_resource(two_gun_registry());
    with_shot_log(&mut app);

    // Two PLAYER reactors in column x=4 (rays to the mover's row never re-cross the
    // column, so neither occludes the other): the MOUNTED-EMPTY one at (4,4) (its
    // emplacement north at (4,3), out of every eastward sightline) and the
    // CARRIED-ELIGIBLE one at (4,7). One tough ENEMY mover walks east through both
    // sightlines from (7,5).
    let mounted_cell = ground(4, 4);
    let eligible_cell = ground(4, 7);
    let mover_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(mounted_cell, PLAYER, Direction::East),
            watcher(eligible_cell, PLAYER, Direction::East),
            tough_mover(mover_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(mounted_reactor), Some(eligible), Some(mover)) = (
        ganger_at(&mut app, mounted_cell),
        ganger_at(&mut app, eligible_cell),
        ganger_at(&mut app, mover_start),
    ) else {
        unreachable!("setup spawns both reactors and the mover at their fixture cells");
    };

    let emplacement = spawn_emplacement(&mut app, ground(4, 3));
    let mount = man_emplacement(&mut app, mounted_reactor, emplacement);
    empty_magazine(&mut app, mount);
    assert!(
        carried_gun_loaded(&mut app, mounted_reactor, mount),
        "fixture precondition: the mounted reactor's carried gun is loaded",
    );

    // The act: one mover steps in BOTH reactors' sightlines the same pass.
    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    step(&mut app, 16);

    // The carried-eligible reactor fires exactly once and spends exactly once.
    assert_eq!(
        shots_by(&app, eligible),
        1,
        "the carried-eligible reactor dispatched exactly one interrupt (cap == 1)",
    );
    assert_eq!(
        used_of(&app, eligible),
        Some(1),
        "the carried-eligible reactor's counter moved exactly once — 1:1 with its shot",
    );
    // The mounted-empty reactor: no shot, no cap spend.
    assert_eq!(
        shots_by(&app, mounted_reactor),
        0,
        "the mounted-empty reactor dispatched no reaction shot",
    );
    assert_eq!(
        used_of(&app, mounted_reactor),
        Some(0),
        "GTW-660: the mounted-empty reactor's ReactionsUsed counter is untouched",
    );

    // The extended GTW-646 invariant across the mixed tick: Σ used == Σ dispatched.
    let total_used: u32 = [mounted_reactor, eligible]
        .into_iter()
        .filter_map(|entity| used_of(&app, entity))
        .sum();
    let total_shots: usize = [mounted_reactor, eligible]
        .into_iter()
        .map(|entity| shots_by(&app, entity))
        .sum();
    assert_eq!(
        u32::try_from(total_shots).ok(),
        Some(total_used),
        "GTW-660 / GTW-646: ReactionsUsed increments correspond 1:1 with dispatched \
         reaction shots across the mounted-empty + carried-eligible mixed tick",
    );
}
