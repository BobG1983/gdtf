//! The arrangement the ejection tests share: a crouched gunner on a mount about to fall.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::Direction,
    metric::CellLevel,
    situation::GangerSpawn,
    test_support::{SituationBuilder, emplacement_at},
};

use super::{
    app::{drive_setup, ground},
    reads::occupant_band,
    spawns::{crouching_player_at, mount, player_on, seated_emplacement},
    weapons::wields_mount,
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
    let situation = SituationBuilder::new()
        .with_ganger(crouching_player_at(entry_cell(), Direction::East))
        .with_gangers(extras)
        .with_scatter(emplacement_at(seat_cell()))
        .build_with_gangs();
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
