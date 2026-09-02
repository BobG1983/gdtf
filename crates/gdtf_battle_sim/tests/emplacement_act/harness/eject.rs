//! The arrangement the ejection tests share: a crouched gunner on a mount about to fall.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::Direction,
    metric::CellLevel,
    situation::{CoverSpawn, GangerSpawn},
    terrain::{def::TerrainUuid, emplacement::MountedWeaponEntity, facing::TerrainFacing},
    test_support::{SituationBuilder, test_pieces},
};

use super::{
    app::{drive_setup, ground},
    reads::{occupant_band, pos_of},
    spawns::{crouching_player_at, mount, player_on, seated_emplacement},
    weapons::{OWN_DAMAGE_TYPE, firing_damage_type, wields_mount},
};

/// The cell the emplacement stands on in every ejection test.
pub(crate) fn seat_cell() -> CellLevel {
    ground(6, 5)
}

/// The cell its gunner enters from, and the cell a free ejection puts it back on.
pub(crate) fn entry_cell() -> CellLevel {
    ground(5, 5)
}

/// A manned seat and the gunner riding it.
pub(crate) struct MannedSeat {
    /// The emplacement entity.
    pub(crate) emplacement: Entity,
    /// The ganger manning it.
    pub(crate) occupant:    Entity,
}

/// Seat a crouched gunner on a mount any impact destroys, alongside whatever else spawns.
pub(crate) fn manned_seat(app: &mut App, seed: u64, extras: Vec<GangerSpawn>) -> MannedSeat {
    manned_seat_of(app, seed, extras, test_pieces::EMPLACEMENT, Vec::new())
}

/// The same seat, built from a named emplacement def, plus any walls the case needs.
pub(crate) fn manned_seat_of(
    app: &mut App,
    seed: u64,
    extras: Vec<GangerSpawn>,
    piece: TerrainUuid,
    walls: Vec<CellLevel>,
) -> MannedSeat {
    let mut builder = SituationBuilder::new()
        .with_ganger(crouching_player_at(entry_cell(), Direction::East))
        .with_gangers(extras)
        .with_scatter(CoverSpawn::new(seat_cell(), piece, TerrainFacing::North));
    for at in walls {
        builder = builder.wall_at(at);
    }
    let situation = builder.build_with_gangs();
    drive_setup(app, seed, situation);
    let emplacement = seated_emplacement(app, seat_cell());
    let occupant = player_on(app, entry_cell());
    mount(app, occupant, emplacement);
    assert!(
        wields_mount(app, occupant),
        "PRECONDITION: the gunner must wield the mount, or the weapon read after the ejection \
         reports a gun the seat never handed out",
    );
    assert_eq!(
        occupant_band(app, seat_cell()),
        Some(HeightBand::Mid),
        "PRECONDITION: a crouched gunner reads Mid on the seat, or a high round strikes it \
         instead of the mount; it reads {:?}",
        occupant_band(app, seat_cell()),
    );
    seed_failing_mount(app, seat_cell());
    MannedSeat {
        emplacement,
        occupant,
    }
}

/// Replace the mount's ledger entry with one hit point and no armour, so any impact fells it.
fn seed_failing_mount(app: &mut App, at: CellLevel) {
    let entry = CoverEntry::seeded(
        CoverHp::new(1),
        HeightBand::High,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Emplacement,
    );
    app.world_mut()
        .resource_mut::<CoverLedger>()
        .insert(at, entry);
}

/// Whether the ledger records the piece at this cell as destroyed.
pub(crate) fn mount_destroyed(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .is_some_and(|entry| *entry.destroyed)
}

/// Whether any entity in the world still records a mounted weapon.
pub(crate) fn any_mount_recorded(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut query = world.query::<&MountedWeaponEntity>();
    query.iter(world).next().is_some()
}

/// The whole result of an ejection: the piece is gone, and the gunner stands on `landing`.
pub(crate) fn assert_ejected(app: &mut App, seat: &MannedSeat, landing: CellLevel) {
    assert!(
        app.world().get_entity(seat.emplacement).is_err(),
        "the destroyed emplacement is despawned once the ejection has read its seat",
    );
    assert!(
        !any_mount_recorded(app),
        "the mounted weapon record goes with the piece, so nothing in the world still names one",
    );
    assert_eq!(
        firing_damage_type(app, seat.occupant),
        Some(OWN_DAMAGE_TYPE),
        "the ejected gunner fires its own carried gun again, not the mount; it would fire {:?}",
        firing_damage_type(app, seat.occupant),
    );
    assert_eq!(
        pos_of(app, seat.occupant),
        Some(landing),
        "the ejected gunner stands on {landing:?}; it stands on {:?}",
        pos_of(app, seat.occupant),
    );
}
