//! Move-sync slot bookkeeping (C9a/C3) + the [`PrevSlot`] tracking it relies on.

use super::{super::PrevSlot, support::*};
use crate::{
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
};

/// C9(a) — a ganger MOVES: one tick after mutating [`Position`], the OLD slot
/// is cleared and the NEW slot is marked, WITHOUT any full-grid-rebuild call.
///
/// Spawns a ganger at an initial cell, ticks once (initial placement marks the
/// start slot via the first-run `Changed` semantics), then mutates `Position`
/// to a new cell and ticks again. Asserts the start slot is now empty and the
/// new slot holds the entity — the in-place clear-old + mark-new of C3. The
/// grid is only ever maintained via the systems; `build_from_occupancy_input`
/// is never called.
#[test]
fn moved_ganger_clears_old_slot_and_marks_new() {
    let mut app = headless_app();
    let start = key(5, 6, 0);
    let dest = key(9, 2, 1);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(start),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    // First tick: initial placement (Position reads as Changed on first run).
    app.update();
    assert_eq!(
        grid_occupant(&app, start),
        Some(ganger),
        "initial placement must mark the start slot",
    );
    // GTW-304 — the occupant's silhouette band is published TOGETHER with the
    // occupant (a standing ganger presents the HIGH band).
    assert_eq!(
        grid_band(&app, start),
        Some(HeightBand::High),
        "initial placement must publish the stance-derived band (standing → HIGH)",
    );

    // Move it: mutate Position, then tick once.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, start),
        None,
        "the OLD slot must be cleared after a move (C3)",
    );
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "the NEW slot must be marked after a move (C3)",
    );
    // The band moves WITH the occupant: cleared from the old slot, published at new.
    assert_eq!(
        grid_band(&app, start),
        None,
        "the OLD slot's band must be cleared after a move (GTW-304)",
    );
    assert_eq!(
        grid_band(&app, dest),
        Some(HeightBand::High),
        "the NEW slot must carry the band after a move (GTW-304)",
    );
}

/// Two consecutive moves keep the grid consistent — the second move clears the
/// FIRST destination (now the tracked previous slot), not the original start.
/// Proves the [`PrevSlot`] bookkeeping advances with each move.
#[test]
fn two_moves_track_the_previous_slot() {
    let mut app = headless_app();
    let a = key(1, 1, 0);
    let b = key(2, 2, 0);
    let c = key(3, 3, 0);

    let ganger = app
        .world_mut()
        .spawn((Position::new(a), LifeState::Alive))
        .id();
    app.update();

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(b);
    }
    app.update();

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(c);
    }
    app.update();

    assert_eq!(grid_occupant(&app, a), None, "the original start is empty");
    assert_eq!(
        grid_occupant(&app, b),
        None,
        "the first destination is empty"
    );
    assert_eq!(
        grid_occupant(&app, c),
        Some(ganger),
        "only the latest destination holds the ganger",
    );
}

/// The [`PrevSlot`] newtype round-trips the slot it records — the bookkeeping
/// the move system relies on.
#[test]
fn prev_slot_round_trips() {
    let slot = key(4, 5, 6);
    assert_eq!(PrevSlot::new(slot).slot(), slot);
    assert_eq!(PrevSlot::new(slot).upper(), None, "new() has no upper");
}

/// GTW-391: [`PrevSlot::with_upper`] round-trips both the lower and upper cells.
#[test]
fn prev_slot_with_upper_round_trips() {
    let lower = key(4, 5, 2);
    let upper = key(4, 5, 3);
    let ps = PrevSlot::with_upper(lower, upper);
    assert_eq!(ps.slot(), lower, "slot() returns the lower cell");
    assert_eq!(
        ps.upper(),
        Some(upper),
        "upper() returns the recorded upper cell"
    );
}
