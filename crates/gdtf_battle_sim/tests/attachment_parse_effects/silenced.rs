//! The `Silenced` integration seam — a silenced shot yields no suppression where an
//! un-silenced one does, and the shared gate reads the wielded ranged weapon tag.

use bevy::{
    app::App,
    ecs::system::RunSystemOnce,
    prelude::{Entity, MessageReader, Resource},
};
use gdtf_battle_sim::{
    effects::attachments::AttachmentEffect,
    ganger::Direction,
    prelude::Cell,
    suppression::SuppressionApplied,
    test_support::SituationBuilder,
    weapon::{
        FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Silenced,
        shooter_weapon_silenced,
    },
};

use super::harness::*;

// ── The Silenced dual-producer gate: a silenced shot makes no suppression ────────

/// A test-local recorder of every `SuppressionApplied` observed across the run.
#[derive(Resource, Default)]
struct AppliedLog {
    /// The number of `SuppressionApplied` signals observed.
    count: usize,
}

/// Drain `SuppressionApplied` into the recorder (registered after `BattleSimPlugin`).
fn record_applied(
    mut msgs: MessageReader<SuppressionApplied>,
    mut log: bevy::prelude::ResMut<AppliedLog>,
) {
    for _msg in msgs.read() {
        log.count += 1;
    }
}

/// Build a two-ganger app where a PLAYER point-blank-fires at an ADJACENT enemy, recording
/// `SuppressionApplied`. The player's weapon carries `effects`.
fn suppression_probe_app(effects: Vec<AttachmentEffect>) -> (App, Entity) {
    let mut app = battle_app(effects);
    app.init_resource::<AppliedLog>();
    app.add_systems(bevy::app::Update, record_applied);
    // A radius-1 suppression disc reaches the adjacent enemy at (6,5).
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            enemy_at(ground(6, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(shooter) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns one player");
    };
    (app, shooter)
}

#[test]
fn silenced_shot_produces_no_suppression_where_an_unsilenced_shot_does() {
    // Un-silenced control: the enemy within the suppression radius IS suppressed.
    let (mut loud, loud_shooter) = suppression_probe_app(Vec::new());
    loud.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            loud_shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    for _ in 0..4 {
        loud.update();
    }
    let loud_count = loud.world().resource::<AppliedLog>().count;
    assert!(
        loud_count > 0,
        "an UN-silenced shot suppresses the adjacent enemy (control: {loud_count} signals)",
    );

    // Silenced: the SAME point-blank shot produces NO SuppressionApplied.
    let (mut quiet, quiet_shooter) = suppression_probe_app(vec![AttachmentEffect::Silence]);
    quiet
        .world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            quiet_shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    for _ in 0..4 {
        quiet.update();
    }
    assert_eq!(
        quiet.world().resource::<AppliedLog>().count,
        0,
        "a SILENCED shot produces NO SuppressionApplied (the producer gates on the shooter's \
         Silenced weapon)",
    );
}

/// The single-shot mode matching the test weapon's authored mode.
const fn single_shot_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

// ── The shared silenced gate helper resolves the shooter's ranged weapon ─────────

#[test]
fn shooter_weapon_silenced_reads_the_wielded_ranged_weapon_tag() {
    // A silenced-wielding shooter reads `true`; an un-silenced one reads `false` — proving
    // the shared `shooter → Wields → the ranged weapon → Silenced` resolution the two
    // producers gate on.
    let (mut silenced_app, silenced_shooter) =
        suppression_probe_app(vec![AttachmentEffect::Silence]);
    let is_silenced = silenced_app
        .world_mut()
        .run_system_once_with(silenced_probe, silenced_shooter)
        .unwrap_or(false);
    assert!(
        is_silenced,
        "shooter_weapon_silenced is true for a shooter wielding a Silenced weapon",
    );

    let (mut loud_app, loud_shooter) = suppression_probe_app(Vec::new());
    let is_loud = loud_app
        .world_mut()
        .run_system_once_with(silenced_probe, loud_shooter)
        .unwrap_or(true);
    assert!(
        !is_loud,
        "shooter_weapon_silenced is false for a shooter wielding an un-silenced weapon",
    );
}

/// A one-shot system exercising the shared `shooter_weapon_silenced` gate over the live
/// wield / melee-probe / Silenced-marker queries — the EXACT resolution the two producers use.
fn silenced_probe(
    shooter: bevy::prelude::In<Entity>,
    wields: gdtf_battle_sim::fire::WieldsQuery,
    melee: gdtf_battle_sim::fire::MeleeQuery,
    silenced: bevy::prelude::Query<(), bevy::prelude::With<Silenced>>,
) -> bool {
    shooter_weapon_silenced(*shooter, &wields, &melee, &silenced)
}
