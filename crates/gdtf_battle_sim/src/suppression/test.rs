use bevy::prelude::App;

use super::stance::stance_for_cover_band;
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Position, Stance, StanceKind, Suppressed, SuppressorCell},
    metric::{Cell, CellLevel, Level},
    terrain::entity::TerrainPieceKind,
    test_support::SimAppBuilder,
};

fn seed_cover(app: &mut App, at: CellLevel) {
    if let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() {
        ledger.insert(
            at,
            CoverEntry::seeded(
                CoverHp::new(10),
                HeightBand::Mid,
                ArmorProtection::new(0),
                ArmorHardness::new(0),
                TerrainPieceKind::Cover,
            ),
        );
    }
}

#[test]
fn stance_for_cover_band_maps_each_band() {
    assert_eq!(stance_for_cover_band(HeightBand::Low), StanceKind::Prone);
    assert_eq!(
        stance_for_cover_band(HeightBand::Mid),
        StanceKind::Crouching
    );
    assert_eq!(
        stance_for_cover_band(HeightBand::High),
        StanceKind::Crouching
    );
}

#[test]
fn auto_stance_reads_cover_on_the_units_storey_not_the_suppressors() {
    let mut app = SimAppBuilder::new().with_acts().build();
    let unit_at = CellLevel::new(Cell::new(10, 10), Level::new(1));
    let suppressor = CellLevel::new(Cell::new(20, 10), Level::new(0));
    seed_cover(&mut app, CellLevel::new(Cell::new(11, 10), Level::new(1)));
    app.update();

    let unit = app
        .world_mut()
        .spawn((
            Position::new(unit_at),
            Stance::new(StanceKind::Standing),
            Suppressed::new(SuppressorCell::new(suppressor)),
        ))
        .id();
    app.update();

    assert_eq!(
        app.world().get::<Stance>(unit).copied(),
        Some(Stance::new(StanceKind::Crouching)),
        "the auto-stance drop reads the cover cell toward the suppressor on the unit's own \
         storey, so a suppressor a storey below still drops the unit behind the cover beside it",
    );
}
