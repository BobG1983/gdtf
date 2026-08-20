//! A mounted reactor whose MOUNT cannot fire is skipped, and never charged for its carried gun.

use gdtf_battle_sim::{acts::MoveRequested, ganger::Direction, test_support::SituationBuilder};

use super::{harness::*, support::*};

#[test]
fn empty_mounted_gun_offer_is_skipped_without_cap_spend() {
    let mut app = battle_app(forced_reaction_tuning(1));
    app.insert_resource(two_gun_registry());
    with_shot_log(&mut app);

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

    let emplacement = spawn_emplacement(&mut app, ground(5, 6));
    let mount = man_emplacement(&mut app, reactor, emplacement);
    empty_magazine(&mut app, mount);
    assert!(
        carried_gun_loaded(&mut app, reactor, mount),
        "fixture precondition: the carried gun is loaded (only the mount is empty)",
    );

    settle_the_enter_exchange(&mut app, &[reactor], mover);
    let baseline = shots_by(&app, reactor);
    let walks_from = walking_from(&app, mover);

    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 4)));
    step(&mut app, 16);

    assert_the_mover_walked(&app, mover, mover_start);
    assert_the_mover_walked(&app, mover, walks_from);
    assert_eq!(
        shots_by(&app, reactor) - baseline,
        0,
        "the empty mounted gun dispatches no interrupt shot",
    );
    assert_eq!(
        used_of(&app, reactor),
        Some(0),
        "a mounted reactor whose MOUNT cannot fire is skipped before the \
         roll — never cap-charged on its CARRIED gun's eligibility",
    );
}

#[test]
fn mixed_tick_mounted_empty_and_carried_eligible_spend_tracks_shots() {
    let mut app = battle_app(forced_reaction_tuning(1));
    app.insert_resource(two_gun_registry());
    with_shot_log(&mut app);

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

    settle_the_enter_exchange(&mut app, &[mounted_reactor, eligible], mover);
    let mounted_baseline = shots_by(&app, mounted_reactor);
    let eligible_baseline = shots_by(&app, eligible);
    let walks_from = walking_from(&app, mover);

    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    step(&mut app, 16);

    assert_the_mover_walked(&app, mover, mover_start);
    assert_the_mover_walked(&app, mover, walks_from);
    let eligible_shots = shots_by(&app, eligible) - eligible_baseline;
    let mounted_shots = shots_by(&app, mounted_reactor) - mounted_baseline;
    assert_eq!(
        eligible_shots, 1,
        "the carried-eligible reactor dispatched exactly one interrupt (cap == 1)",
    );
    assert_eq!(
        used_of(&app, eligible),
        Some(1),
        "the carried-eligible reactor's counter moved exactly once — 1:1 with its shot",
    );
    assert_eq!(
        mounted_shots, 0,
        "the mounted-empty reactor dispatched no reaction shot",
    );
    assert_eq!(
        used_of(&app, mounted_reactor),
        Some(0),
        "the mounted-empty reactor's ReactionsUsed counter is untouched",
    );

    let total_used: u32 = [mounted_reactor, eligible]
        .into_iter()
        .filter_map(|entity| used_of(&app, entity))
        .sum();
    assert_eq!(
        u32::try_from(mounted_shots + eligible_shots).ok(),
        Some(total_used),
        "ReactionsUsed increments correspond 1:1 with dispatched \
         reaction shots across the mounted-empty + carried-eligible mixed tick",
    );
}
