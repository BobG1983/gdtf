//! The battle-setup driver, the emplacement toggle, and the ganger the presenter draws.

use bevy::{
    app::{App, Update},
    ecs::{entity::Entity, message::Messages},
};
use cobalt_test_utils::advance_until_resource_exists;
use gdtf_battle_sim::{
    battle::{SetupBattleRequested, setup_battle_on_request},
    emplacement::{SetEmplacement, apply_emplacement_toggle},
    ganger::{Aiming, Facing, Hp, LifeState, Position, Stance, Tu, Wounds},
    prelude::{CellLevel, Faction},
    rng::{BattleSeed, ShotRng},
    situation::{PlacedGanger, Situation},
    test_support::setup_request,
};

const SEED: u64 = 0x0D15_EA5E;

/// Register the sim systems a driven battle setup and an emplacement occupy need.
pub(super) fn register_setup_driver(app: &mut App) {
    app.add_message::<SetupBattleRequested>()
        .add_message::<SetEmplacement>()
        .add_systems(Update, (setup_battle_on_request, apply_emplacement_toggle));
}

/// Run `setup_battle` on this situation and settle.
pub(crate) fn drive_setup(app: &mut App, built: (Situation, Vec<PlacedGanger>)) {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .write(setup_request(built, BattleSeed::new(SEED)));
    advance_until_resource_exists::<ShotRng>(app);
    app.update();
}

/// Spawn one living ganger the presenter's own spawn system draws a sprite for.
pub(crate) fn spawn_test_ganger(app: &mut App, at: CellLevel) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(at),
            Faction::new(0),
            Facing::default(),
            Stance::default(),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(20),
            Hp::new(10),
            Wounds::new(3),
        ))
        .id()
}
