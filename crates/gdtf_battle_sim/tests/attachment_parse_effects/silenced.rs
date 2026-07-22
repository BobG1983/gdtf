//! The `Silenced` integration — a silenced shot yields no suppression where an
//! un-silenced one does, and the shared gate reads the wielded ranged weapon tag.

use bevy::{
    app::App,
    ecs::system::RunSystemOnce,
    prelude::{Entity, MessageReader, Resource, World},
};
use gdtf_battle_sim::{
    effects::attachments::AttachmentEffect,
    ganger::Direction,
    prelude::Cell,
    suppression::SuppressionApplied,
    test_support::SituationBuilder,
    weapon::{
        FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, MountedWeapon, Silenced,
        WieldedBy, shooter_weapon_silenced,
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

// ── GTW-674: the gate resolves the FIRING weapon (mounted-first), not the carried gun ──

/// Relate a [`MountedWeapon`]-marked weapon entity to an already-spawned `ganger` — the
/// emplacement's bolted-down gun a seated ganger fires (GTW-543). Carries [`Silenced`] iff
/// `silenced` is `true`; otherwise it is a normal, LOUD mount. Because the carried gun is
/// related FIRST (by setup), this mount is the LATER entry in the insertion-ordered `Wields`
/// collection — so the OLD ranged-only gate resolves the CARRIED gun, while the fixed
/// firing-weapon gate PREFERS this mount (the GTW-673 ordering discriminator).
///
/// Single-file consumer, so it lives local to its consumers (module-layout rule 6). The gate
/// probes only `With<MountedWeapon>` / `With<Silenced>`, so a marker-only mount entity is the
/// exact world state the silenced gate reads.
fn man_mount(world: &mut World, ganger: Entity, silenced: bool) {
    if silenced {
        world.spawn((WieldedBy::new(ganger), MountedWeapon, Silenced::new(true)));
    } else {
        world.spawn((WieldedBy::new(ganger), MountedWeapon));
    }
}

/// Write a point-blank [`FireRequested`](gdtf_battle_sim::acts::FireRequested) from `shooter`
/// at the adjacent enemy cell and settle it, returning the count of `SuppressionApplied`
/// signals observed — the LOUD-vs-silent outcome the suppression producer keys off.
fn suppress_and_count(app: &mut App, shooter: Entity) -> usize {
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    for _ in 0..4 {
        app.update();
    }
    app.world().resource::<AppliedLog>().count
}

#[test]
fn mounted_shooter_with_silenced_carried_gun_but_loud_mount_still_suppresses() {
    // The shooter's CARRIED gun is silenced (the attachment effect), but it MANS an un-silenced
    // mount — the weapon dispatch would actually fire. The suppression producer must resolve the
    // FIRING weapon (the loud mount) and produce the LOUD outcome.
    //
    // Pin-discriminating against the UNFIXED gate: it resolved the carried RANGED weapon (the
    // silenced gun, related first) and SWALLOWED the shot — zero suppression. This assertion
    // therefore flips red against the pre-fix code and green once the gate prefers the mount.
    let (mut app, shooter) = suppression_probe_app(vec![AttachmentEffect::Silence]);
    man_mount(app.world_mut(), shooter, false);
    let count = suppress_and_count(&mut app, shooter);
    assert!(
        count > 0,
        "a MOUNTED shooter firing a LOUD mount suppresses the adjacent enemy even though its \
         CARRIED gun is silenced (the gate resolves the firing weapon, not the carried gun): \
         {count} signals",
    );
}

#[test]
fn mounted_shooter_with_silenced_mount_but_loud_carried_gun_is_silenced() {
    // The converse: the CARRIED gun is loud but the MANNED mount is silenced — the shot the
    // shooter actually fires comes from the silenced mount, so NO suppression is produced.
    let (mut app, shooter) = suppression_probe_app(Vec::new());
    man_mount(app.world_mut(), shooter, true);
    let count = suppress_and_count(&mut app, shooter);
    assert_eq!(
        count, 0,
        "a MOUNTED shooter firing a SILENCED mount makes no suppression even though its CARRIED \
         gun is loud (the gate resolves the firing weapon, not the carried gun): {count} signals",
    );
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
/// wield / mounted-probe / melee-probe / Silenced-marker queries — the EXACT resolution the
/// two producers use (GTW-674: firing-weapon, mounted-first).
fn silenced_probe(
    shooter: bevy::prelude::In<Entity>,
    wields: gdtf_battle_sim::fire::WieldsQuery,
    mounted: gdtf_battle_sim::fire::MountedQuery,
    melee: gdtf_battle_sim::fire::MeleeQuery,
    silenced: bevy::prelude::Query<(), bevy::prelude::With<Silenced>>,
) -> bool {
    *shooter_weapon_silenced(*shooter, &wields, &mounted, &melee, &silenced)
}

#[test]
fn shooter_weapon_silenced_reads_the_firing_weapon_mount_over_the_carried_gun() {
    // GTW-674 direct-gate: with a mount manned, the gate reads the MOUNT's Silenced state, not
    // the carried gun's — the firing-weapon (mounted-first) resolution.

    // A silenced CARRIED gun + a LOUD mount reads FALSE (loud): the gate resolves the mount.
    let (mut loud_mount, loud_shooter) = suppression_probe_app(vec![AttachmentEffect::Silence]);
    man_mount(loud_mount.world_mut(), loud_shooter, false);
    let reads_silenced = loud_mount
        .world_mut()
        .run_system_once_with(silenced_probe, loud_shooter)
        .unwrap_or(true);
    assert!(
        !reads_silenced,
        "the gate reads the LOUD mount (false), not the silenced carried gun",
    );

    // A loud CARRIED gun + a SILENCED mount reads TRUE (silent): the gate resolves the mount.
    let (mut silent_mount, silent_shooter) = suppression_probe_app(Vec::new());
    man_mount(silent_mount.world_mut(), silent_shooter, true);
    let reads_silenced = silent_mount
        .world_mut()
        .run_system_once_with(silenced_probe, silent_shooter)
        .unwrap_or(false);
    assert!(
        reads_silenced,
        "the gate reads the SILENCED mount (true), not the loud carried gun",
    );
}
