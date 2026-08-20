//! What the toggle does with a request it cannot serve, and that it does it the same way twice.

use bevy::prelude::Entity;

use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::StanceKind,
    metric::CellLevel,
    terrain::{
        emplacement::{EmplacementState, SetEmplacement},
        entity::TerrainCell,
    },
};

#[test]
fn toggle_panic_free_on_non_emplacement() {
    let mut app = headless_app();
    let plain: Entity = app.world_mut().spawn(TerrainCell::new(key(9, 9, 0))).id();
    let ganger = stanced_ganger(&mut app, StanceKind::Standing);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(plain, ganger));
    assert!(
        state(&app, plain).is_none(),
        "a non-emplacement entity gains no EmplacementState from a stray SetEmplacement (panic-free)",
    );
}

#[test]
fn toggle_is_deterministic() {
    let at = key(3, 3, 0);
    let start = key(1, 3, 0);
    let run = || -> (Option<EmplacementState>, Option<HeightBand>, Option<CellLevel>) {
        let mut app = headless_app();
        let emplacement = spawn_vacant_emplacement(&mut app, at);
        let ganger = stanced_ganger_at(&mut app, StanceKind::Standing, start);
        app.update();
        toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
        toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
        (
            state(&app, emplacement),
            occupant_band(&app, start),
            position_of(&app, ganger),
        )
    };
    assert_eq!(
        run(),
        run(),
        "identical spawn+toggle sequences yield identical end state (deterministic)",
    );
    assert_eq!(
        run(),
        (
            Some(EmplacementState::Vacant),
            Some(HeightBand::High),
            Some(start),
        ),
        "the settled end state is Vacant with the standing ganger back on the cell it entered \
         from, band and all",
    );
}
