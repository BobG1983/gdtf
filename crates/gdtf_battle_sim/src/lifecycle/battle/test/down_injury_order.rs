use bevy::prelude::Entity;

use super::{
    injury::{
        aim_debuff_catalog, duel_entities, duel_situation, paper_armor_registry,
        penetrating_weapon_registry,
    },
    support::*,
};
use crate::{
    act_log::{ActDeed, ActLog, ActSeq},
    ganger::{Hp, Position},
    injuries::InflictedInjuries,
};

struct DownInjuryRun {
    downed:            bool,
    ledger_has_injury: bool,
    injured_index:     Option<usize>,
    downed_index:      Option<usize>,
}

fn run_down_injury(seed: u64) -> DownInjuryRun {
    let mut app = headless_app();
    app.insert_resource(penetrating_weapon_registry());
    app.insert_resource(paper_armor_registry());
    let (registry, tables) = aim_debuff_catalog();
    app.insert_resource(registry);
    app.insert_resource(tables);
    app.world_mut()
        .write_message(setup_request(duel_situation(), BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }

    let (shooter, target) = duel_entities(&mut app);
    if let Some(mut hp) = app.world_mut().get_mut::<Hp>(target) {
        *hp = Hp::new(1);
    }
    let target_cell = app
        .world()
        .get::<Position>(target)
        .map_or(Cell::new(10, 5), |p| p.cell());

    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    );
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        target_cell,
        Level::new(0),
    ));
    app.update();

    let downed = matches!(
        app.world().get::<LifeState>(target),
        Some(LifeState::Downed)
    );
    let ledger_has_injury = app
        .world()
        .get::<InflictedInjuries>(target)
        .is_some_and(|ledger| !ledger.gained().is_empty());
    let (injured_index, downed_index) = act_log_indices(&app, target);
    DownInjuryRun {
        downed,
        ledger_has_injury,
        injured_index,
        downed_index,
    }
}

fn act_log_indices(app: &App, target: Entity) -> (Option<usize>, Option<usize>) {
    let Some(log) = app.world().get_resource::<ActLog>() else {
        return (None, None);
    };
    let mut injured = None;
    let mut downed = None;
    for (index, entry) in log.since(ActSeq::START).enumerate() {
        if entry.actor() != target {
            continue;
        }
        match *entry.deed() {
            ActDeed::Injured { .. } => injured = injured.or(Some(index)),
            ActDeed::LifeChanged {
                to: LifeState::Downed,
                ..
            } => downed = downed.or(Some(index)),
            _ => {}
        }
    }
    (injured, downed)
}

#[test]
fn a_downing_hit_records_its_injury_before_the_down() {
    let mut found = None;
    for seed in [
        0xA1u64, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6, 0x17, 0x28, 0x39, 0x4A, 0x5B, 0x6C, 0x7D, 0x8E,
    ] {
        let run = run_down_injury(seed);
        if run.downed && run.ledger_has_injury {
            found = Some(run);
            break;
        }
    }
    let Some(run) = found else {
        unreachable!(
            "across the seed sweep, at least one single round must BOTH down the one-HP target \
             and draw a named injury — the same-shot down+injury the regression is about"
        );
    };

    assert!(
        run.downed,
        "the round depleted the target's Hp to 0 -> LifeState::Downed",
    );
    assert!(
        run.ledger_has_injury,
        "the SAME round drew a named injury into the target's InflictedInjuries ledger",
    );

    let Some(injured_index) = run.injured_index else {
        unreachable!("a downed+injured run must have recorded an Injured deed for the target");
    };
    let Some(downed_index) = run.downed_index else {
        unreachable!(
            "a downed+injured run must have recorded a LifeChanged -> Downed for the target"
        );
    };
    assert!(
        injured_index < downed_index,
        "the injury (append index {injured_index}) must be logged BEFORE the down (append \
         index {downed_index}) — record_acts records consequence(injury) before life(Downed); \
         a reversal is the defect",
    );
}
