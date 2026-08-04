use bevy::app::App;
use gdtf_battle_sim::{
    acts::{EndTurnRequested, FireRequested},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, Destroyed, HeightBand},
    ganger::{Direction, Suppressed},
    metric::{Cell, CellLevel, Level},
    prelude::StanceKind,
    rng::ReactionRng,
    test_support::{SituationBuilder, reaction_rng},
};

use super::harness::*;

fn run_until_active(app: &mut App, faction: u8) {
    for _ in 0..96 {
        app.update();
        if active_faction_is(app, faction) {
            return;
        }
    }
}

const fn cover_at_band(band: HeightBand) -> CoverEntry {
    CoverEntry {
        current_hp:       CoverHp::new(30),
        max_hp:           CoverHp::new(30),
        height_band:      band,
        armor_protection: ArmorProtection::new(2),
        armor_hardness:   ArmorHardness::new(1),
        destroyed:        Destroyed::new(false),
    }
}

const fn mid_cover() -> CoverEntry {
    cover_at_band(HeightBand::Mid)
}

const fn low_cover() -> CoverEntry {
    cover_at_band(HeightBand::Low)
}

fn seed_cover(app: &mut App, cell: CellLevel, entry: CoverEntry) {
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        unreachable!("BattleSimPlugin inserts the CoverLedger at setup");
    };
    ledger.insert(cell, entry);
}

#[test]
fn suppressed_reactor_consumes_zero_reaction_rng_draws() {
    let fresh_first_draw = reaction_rng(SEED).next_u64();

    let build = |suppress_watcher: bool| {
        let mut app = battle_app(0);
        let situation = SituationBuilder::new()
            .with_gangers([
                ganger(ground(6, 5), ENEMY, Direction::East),
                ganger(ground(4, 5), PLAYER, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(actor), Some(watcher)) =
            (ganger_of(&mut app, ENEMY), ganger_of(&mut app, PLAYER))
        else {
            unreachable!("one enemy actor + one player watcher spawned");
        };
        if suppress_watcher {
            app.world_mut().entity_mut(watcher).insert(Suppressed::new(
                gdtf_battle_sim::ganger::SuppressorCell::new(ground(6, 5)),
            ));
            step(&mut app, 1);
        }
        let mode = wielded_single_mode(&mut app, actor);
        app.world_mut().write_message(FireRequested::new(
            actor,
            mode,
            Cell::new(9, 5),
            Level::new(0),
        ));
        step(&mut app, 4);
        let watcher_tu = tu_of(&app, watcher);
        let next_draw = app
            .world_mut()
            .get_resource_mut::<ReactionRng>()
            .map(|mut r| r.next_u64());
        (watcher, watcher_tu, next_draw)
    };

    let (_control_watcher, control_tu, control_next) = build(false);
    let Some(control_tu) = control_tu else {
        unreachable!("the control watcher persists");
    };
    let Some(control_next) = control_next else {
        unreachable!("the ReactionRng stream is present in the control");
    };

    let (_supp_watcher, supp_tu, supp_next) = build(true);
    let Some(supp_tu) = supp_tu else {
        unreachable!("the suppressed watcher persists");
    };
    let Some(supp_next) = supp_next else {
        unreachable!("the ReactionRng stream is present in the suppressed run");
    };

    assert_eq!(
        supp_next, fresh_first_draw,
        "(b) the suppressed reactor consumed ZERO ReactionRng draws — the world stream still \
         sits at its initial position (next draw == a fresh stream's first draw)",
    );
    assert_ne!(
        control_next, fresh_first_draw,
        "(b) precondition: the unsuppressed control reactor DID draw (its interrupt roll \
         advanced the stream past the fresh first draw) — else the zero-draw claim is vacuous",
    );
    assert!(
        control_tu < supp_tu,
        "(b) the control watcher's TU debits on its interrupt shot ({control_tu}); the \
         suppressed watcher never fires, so its TU is higher ({supp_tu})",
    );
}

#[test]
fn suppression_clears_at_the_suppressed_units_own_turn_start() {
    let mut app = battle_app(1);
    let target = ground(8, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            ganger(target, PLAYER, Direction::West),
            ganger(ground(4, 5), ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("one player + one enemy spawned");
    };

    let mode = wielded_single_mode(&mut app, enemy);
    app.world_mut().write_message(FireRequested::new(
        enemy,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    assert!(
        is_suppressed(&app, player),
        "(c) precondition: the player is suppressed by the enemy shot",
    );

    app.world_mut().write_message(EndTurnRequested);
    step(&mut app, 3);
    assert!(
        active_faction_is(&app, ENEMY),
        "(c) precondition: the enemy turn is active after the player ends its turn",
    );
    assert!(
        is_suppressed(&app, player),
        "(c) the player stays suppressed through the ENEMY's turn (cleared only at its OWN \
         turn-start, not the opponent's)",
    );

    run_until_active(&mut app, PLAYER);
    assert!(
        active_faction_is(&app, PLAYER),
        "(c) precondition: control cycled back to the player turn",
    );
    step(&mut app, 1);
    assert!(
        !is_suppressed(&app, player),
        "(c) the player's suppression clears at ITS OWN (player) turn-start",
    );
}

#[test]
fn auto_stance_drops_behind_cover_without_charging_tu() {
    let run = |cover: Option<CoverEntry>| -> (Option<StanceKind>, Option<u8>) {
        let mut app = battle_app(0);
        let unit_cell = ground(8, 5);
        let cover_cell = ground(7, 5);
        let situation = SituationBuilder::new()
            .with_gangers([
                ganger(unit_cell, PLAYER, Direction::East),
                ganger(ground(4, 5), ENEMY, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        if let Some(entry) = cover {
            seed_cover(&mut app, cover_cell, entry);
        }
        let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("one player + one enemy spawned");
        };
        let tu_before = tu_of(&app, player);
        let mode = wielded_single_mode(&mut app, enemy);
        app.world_mut().write_message(FireRequested::new(
            enemy,
            mode,
            Cell::new(8, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        assert!(
            is_suppressed(&app, player),
            "auto-stance precondition: the player is suppressed",
        );
        let stance_after = stance_of(&app, player);
        let tu_after = tu_of(&app, player);
        assert_eq!(
            tu_before, tu_after,
            "(d) the auto-stance drop charges NO TU (before {tu_before:?} == after \
             {tu_after:?})",
        );
        (stance_after, tu_after)
    };

    let (low_stance, _) = run(Some(low_cover()));
    assert_eq!(
        low_stance,
        Some(StanceKind::Prone),
        "(d) LOW cover → the unit auto-drops to Prone",
    );

    let (mid_stance, _) = run(Some(mid_cover()));
    assert_eq!(
        mid_stance,
        Some(StanceKind::Crouching),
        "(d) MID cover → the unit auto-drops to Crouching",
    );

    let (none_stance, _) = run(None);
    assert_eq!(
        none_stance,
        Some(StanceKind::Standing),
        "(d) NO adjacent cover → the stance is UNCHANGED (still Standing, the spawn posture)",
    );
}
