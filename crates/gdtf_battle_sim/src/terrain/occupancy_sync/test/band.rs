//! GTW-304 — silhouette-band publication tracking the occupant's stance.

use super::support::*;
use crate::{
    clearance::silhouette_band,
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
};

/// GTW-304 — the published occupant band tracks the ganger's STANCE: a kneeling
/// ganger presents the MID band, a prone ganger the LOW band, and re-posing in
/// place re-publishes the band at the (unchanged) current slot.
#[test]
fn occupant_band_tracks_stance() {
    let mut app = headless_app();
    let at = key(11, 4, 0);

    // Spawn KNEELING — the band must be MID.
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Crouching),
            LifeState::Alive,
        ))
        .id();
    app.update();
    assert_eq!(
        grid_band(&app, at),
        Some(silhouette_band(StanceKind::Crouching)),
        "a kneeling ganger publishes the MID silhouette band",
    );
    assert_eq!(grid_band(&app, at), Some(HeightBand::Mid));

    // Re-pose to PRONE in place (no move) — the band must re-publish as LOW.
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the occupant stays put on an in-place re-pose",
    );
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::Low),
        "re-posing prone re-publishes the LOW band at the unchanged slot (GTW-304)",
    );
}

/// GTW-304 — a ganger going DOWN clears its band along with its occupant marker
/// (occupant and band stay consistent — no slot left banded but un-occupied).
#[test]
fn dead_ganger_clears_its_band() {
    let mut app = headless_app();
    let at = key(14, 15, 1);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();
    app.update();
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::High),
        "the alive ganger's band is published",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a dead ganger's occupant slot must be cleared (C4)",
    );
    assert_eq!(
        grid_band(&app, at),
        None,
        "a dead ganger's band must be cleared too (GTW-304)",
    );
}
