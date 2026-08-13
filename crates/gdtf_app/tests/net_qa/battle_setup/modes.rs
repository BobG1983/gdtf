//! A battle fixture whose shooter's gun offers two modes at prices only one of them can pay.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::cell::CellLevelNet;
use gdtf_battle_input::{ChosenFireMode, SelectedShooter};
use gdtf_battle_sim::{
    acts::fire_arc_tu_cost,
    ganger::{Aiming, Direction, Facing, Position, Tu, TuMax},
    magazine::{Magazine, mode_tu_cost},
    prelude::{Cell, CellLevel},
    tuning::CombatTuning,
    weapon::{
        FireMode, FireModeSpec, Handedness, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    },
};
use gdtf_qa_protocol::ports::NetQaPort;

use super::expected::PosedShooter;
use crate::{
    battle_fixture::arm_selected_with_modes,
    battle_reads::a_player_ganger,
    magazine_support::fill_the_magazine,
    socket_support::{TestError, battle_app_listening},
};

/// The cheap mode this fixture's gun offers, and the one an unset gun reads as.
const CHEAP_SINGLE: FireModeSpec = FireModeSpec::new(
    ModeKind::Single,
    ModeConeMult::new(1.0),
    ModeTuPercent::new(0.2),
    ModeShots::new(1),
);

/// The dear mode this fixture's gun offers, at the same one round so only the price differs.
const DEAR_BURST: FireModeSpec = FireModeSpec::new(
    ModeKind::Burst,
    ModeConeMult::new(1.0),
    ModeTuPercent::new(0.6),
    ModeShots::new(1),
);

/// What the fixture's two modes cost this shooter, and the pool a turn-and-single needs.
struct Priced {
    single: Tu,
    burst:  Tu,
    pool:   Tu,
}

/// A north-facing shooter holding a loaded two-mode gun set to `chosen`, and the cell behind it.
///
/// The shooter is left holding exactly what a turn-and-single costs, so the dearer mode is not
/// affordable and the single is.
pub(crate) fn battle_with_a_two_mode_gun(
    chosen: Option<ModeKind>,
) -> Result<(App, NetQaPort, PosedShooter), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let Some((shooter, at)) = a_player_ganger(&app) else {
        return Err("a generated battle must field at least one player ganger".into());
    };
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    if fill_the_magazine(&mut app, shooter).is_none() {
        return Err("the posed shooter must hold a ranged weapon with a magazine".into());
    }
    let weapon = arm_selected_with_modes(&mut app, FireMode::new(vec![CHEAP_SINGLE, DEAR_BURST]));
    the_read_can_fetch(&app, weapon)?;
    face_north(&mut app, shooter)?;

    let (cell, level) = at.to_sim().split();
    let behind = CellLevel::new(Cell::new(cell.x, cell.y + 1), level);
    let priced = priced_for(&app, shooter, behind)?;
    if *priced.burst <= *priced.single {
        return Err(format!(
            "the fixture's burst must price above its single, or the two cases answer alike for \
             a reason nobody chose: burst {:?} against single {:?}",
            priced.burst, priced.single,
        )
        .into());
    }
    hand_the_shooter(&mut app, shooter, priced.pool)?;
    if let Some(kind) = chosen {
        let Ok(mut gun) = app.world_mut().get_entity_mut(weapon) else {
            return Err("the weapon the fixture just armed must still exist".into());
        };
        gun.insert(ChosenFireMode::new(kind));
    }
    let behind = CellLevelNet::from_sim(behind);
    Ok((app, port, PosedShooter { behind }))
}

/// Fail unless the armed gun carries the columns the sightline read takes as required.
///
/// A gun missing either is a gun the read cannot fetch, and it then answers false for every mode.
fn the_read_can_fetch(app: &App, weapon: Entity) -> Result<(), TestError> {
    let Ok(gun) = app.world().get_entity(weapon) else {
        return Err("the weapon the fixture just armed must still exist".into());
    };
    if !gun.contains::<Magazine>() || !gun.contains::<Handedness>() {
        return Err(
            "the armed gun must carry a magazine and a handedness, or the sightline read \
                    fetches no gun and answers false whatever mode is set"
                .into(),
        );
    }
    Ok(())
}

/// Turn the shooter north, which is what puts the cell behind it outside its firing arc.
fn face_north(app: &mut App, shooter: Entity) -> Result<(), TestError> {
    let Ok(mut row) = app.world_mut().get_entity_mut(shooter) else {
        return Err("the ganger the world just answered with must still exist".into());
    };
    row.insert(Facing::new(Direction::North));
    Ok(())
}

/// Leave the shooter holding `pool` time units.
fn hand_the_shooter(app: &mut App, shooter: Entity, pool: Tu) -> Result<(), TestError> {
    let Ok(mut row) = app.world_mut().get_entity_mut(shooter) else {
        return Err("the ganger the world just answered with must still exist".into());
    };
    row.insert(pool);
    Ok(())
}

/// Price both modes against this shooter, and the north-facing turn-and-single at `behind`.
fn priced_for(app: &App, shooter: Entity, behind: CellLevel) -> Result<Priced, TestError> {
    let world = app.world();
    let (Ok(row), Some(tuning)) = (
        world.get_entity(shooter),
        world.get_resource::<CombatTuning>(),
    ) else {
        return Err("a running battle holds its tuning and the shooter it selected".into());
    };
    let (Some(tu_max), Some(aiming), Some(position)) = (
        row.get::<TuMax>(),
        row.get::<Aiming>(),
        row.get::<Position>(),
    ) else {
        return Err("a deployed ganger carries the pose a shot is priced against".into());
    };
    let single = mode_tu_cost(&CHEAP_SINGLE, tu_max, aiming, tuning);
    Ok(Priced {
        single,
        burst: mode_tu_cost(&DEAR_BURST, tu_max, aiming, tuning),
        pool: fire_arc_tu_cost(
            Direction::North,
            position.cell(),
            behind.cell(),
            single,
            tuning,
        ),
    })
}
