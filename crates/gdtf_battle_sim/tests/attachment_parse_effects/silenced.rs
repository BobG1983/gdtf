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

#[derive(Resource, Default)]
struct AppliedLog {
    count: usize,
}

fn record_applied(
    mut msgs: MessageReader<SuppressionApplied>,
    mut log: bevy::prelude::ResMut<AppliedLog>,
) {
    for _msg in msgs.read() {
        log.count += 1;
    }
}

fn suppression_probe_app(effects: Vec<AttachmentEffect>) -> (App, Entity) {
    let mut app = battle_app(effects);
    app.init_resource::<AppliedLog>();
    app.add_systems(bevy::app::Update, record_applied);
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

const fn single_shot_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

fn man_mount(world: &mut World, ganger: Entity, silenced: bool) {
    if silenced {
        world.spawn((WieldedBy::new(ganger), MountedWeapon, Silenced::new(true)));
    } else {
        world.spawn((WieldedBy::new(ganger), MountedWeapon));
    }
}

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
    let (mut app, shooter) = suppression_probe_app(Vec::new());
    man_mount(app.world_mut(), shooter, true);
    let count = suppress_and_count(&mut app, shooter);
    assert_eq!(
        count, 0,
        "a MOUNTED shooter firing a SILENCED mount makes no suppression even though its CARRIED \
         gun is loud (the gate resolves the firing weapon, not the carried gun): {count} signals",
    );
}

#[test]
fn shooter_weapon_silenced_reads_the_wielded_ranged_weapon_tag() {
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
