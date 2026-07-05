//! GTW-391 — stair upper-presence TEARDOWN/relocation on move, stance, and
//! life-state transitions.

use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
};

/// GTW-391 Test 6: moving OFF a stair clears the upper-cell presence — the
/// stale-registration #1 risk.
#[test]
fn move_off_stair_clears_upper_presence() {
    let mut app = headless_app();
    let stair = key(8, 8, 2);
    let upper = key(8, 8, 3);
    let dest = key(9, 9, 2); // non-stair destination

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    // Verify stair presence was established.
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence must be written on the stair",
    );

    // Move off the stair.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        None,
        "old lower (stair) slot must be cleared",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "old upper slot must be cleared when moving off the stair (GTW-391)",
    );
    assert_eq!(grid_band(&app, upper), None, "old upper band must be gone");
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "new destination must be occupied",
    );
    // dest is non-stair, so no upper presence there.
    assert_eq!(grid_occupant(&app, key(9, 9, 3)), None);
}

/// GTW-391 Test 7: going prone IN PLACE on a stair clears the upper presence —
/// the subtle same-cell-but-stance-changes case.
#[test]
fn go_prone_in_place_on_stair_clears_upper() {
    let mut app = headless_app();
    let stair = key(3, 3, 0);
    let upper = key(3, 3, 1);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence must exist before going prone",
    );

    // Go prone in place.
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        Some(ganger),
        "lower slot stays occupied (still on the stair)",
    );
    assert_eq!(
        grid_band(&app, stair),
        Some(HeightBand::Low),
        "lower slot now carries Low band (Prone)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "upper slot must be cleared when going prone in place (GTW-391 Test 7)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-391 Test 8: moving FROM one stair to ANOTHER stair relocates the upper
/// presence — old upper cleared, new upper written.
#[test]
fn stair_to_stair_move_relocates_upper() {
    let mut app = headless_app();
    let stair_a = key(5, 5, 1);
    let upper_a = key(5, 5, 2);
    let stair_b = key(15, 15, 1);
    let upper_b = key(15, 15, 2);

    mark_stair(&mut app, stair_a);
    mark_stair(&mut app, stair_b);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair_a),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(grid_occupant(&app, upper_a), Some(ganger));

    // Move to the other stair.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(stair_b);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, upper_a),
        None,
        "old upper must be cleared on stair-to-stair move",
    );
    assert_eq!(grid_band(&app, upper_a), None);
    assert_eq!(
        grid_occupant(&app, upper_b),
        Some(ganger),
        "new upper must be written on the new stair (GTW-391)",
    );
    assert_eq!(grid_band(&app, upper_b), Some(HeightBand::Low));
}

/// GTW-391 Test 9: a ganger KILLED (Dead) on a stair clears BOTH lower AND upper
/// cells. GTW-459: only Dead frees the cells — a Downed stair-occupant retains both
/// (covered by [`downed_on_stair_retains_both_cells`]), so this test uses `Dead`.
#[test]
fn dead_on_stair_clears_upper_presence() {
    let mut app = headless_app();
    let stair = key(7, 7, 3);
    let upper = key(7, 7, 4);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence before death",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        None,
        "lower slot must be cleared when DEAD (GTW-459 C1)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "upper slot must also be cleared when killed on a stair (GTW-391 Test 9)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-459 C2 — a ganger DOWNED on a stair RETAINS BOTH the lower AND the upper
/// cell (occupant + band), the stair mirror of [`downed_ganger_retains_its_slot`]:
/// a downed body holds both cells so it keeps blocking movement and occluding fire
/// on the dual-cell stair presence. Only Dead clears them
/// ([`dead_on_stair_clears_upper_presence`]).
#[test]
fn downed_on_stair_retains_both_cells() {
    let mut app = headless_app();
    let stair = key(7, 7, 3);
    let upper = key(7, 7, 4);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence before going down",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        Some(ganger),
        "a downed stair-occupant HOLDS its lower cell (GTW-459 C2)",
    );
    assert_eq!(
        grid_band(&app, stair),
        Some(HeightBand::High),
        "the downed lower cell keeps its stance band, still occluding fire (GTW-459 C2/C5)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "a downed stair-occupant HOLDS its upper cell too (GTW-459 C2)",
    );
    assert_eq!(
        grid_band(&app, upper),
        Some(HeightBand::Low),
        "the downed upper cell keeps its Low band, still occluding fire (GTW-459 C2/C5)",
    );
}
