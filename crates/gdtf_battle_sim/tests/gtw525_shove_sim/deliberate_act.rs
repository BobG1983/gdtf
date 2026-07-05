//! The deliberate shove ACT — TU-costed and wound-free, gated on adjacency + faction, and
//! seed-deterministic in its shove/fall outcome.

use gdtf_battle_sim::{CellLevel, OccupancyGrid, SlabState, SurfaceGrid};

use super::harness::*;

// ── QA(5) — deliberate act: any ganger, adjacency-gated, TU spent, NO wound ─────

/// QA(5a): a deliberate shove spends the shover's TU (the shove act is TU-costed) and deals NO
/// wound (a supported shove leaves the target's Hp/Wounds untouched — pure displacement).
#[test]
fn deliberate_shove_spends_tu_and_deals_no_wound() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let target = shove_ganger(app.world_mut(), ground(6, 5), 1);
    app.update();
    let tu_before = tu_of(&app, shover);
    let (hp_before, wounds_before) = (hp_of(&app, target), wounds_of(&app, target));

    shove_and_settle(&mut app, shover, target);

    assert!(
        tu_of(&app, shover) < tu_before,
        "the deliberate shove spends the shover's TU (a TU-costed act)"
    );
    assert_eq!(
        hp_of(&app, target),
        hp_before,
        "the deliberate shove deals NO HP damage (pure displacement)"
    );
    assert_eq!(
        wounds_of(&app, target),
        wounds_before,
        "the deliberate shove records NO Wound (pure displacement — the fall does the harm)"
    );
}

/// QA(5b): the deliberate shove is GATED — a NON-ADJACENT target (Chebyshev > 1) is a no-op
/// (no move, no TU spent); a SAME-FACTION (ally) target is a no-op.
#[test]
fn deliberate_shove_gates_adjacency_and_faction() {
    // Non-adjacent: target three cells away.
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let far = shove_ganger(app.world_mut(), ground(8, 5), 1);
    app.update();
    let tu_before = tu_of(&app, shover);
    shove_and_settle(&mut app, shover, far);
    assert_eq!(
        pos_of(&app, far),
        Some(ground(8, 5)),
        "a non-adjacent target is not shoved (the 8-adjacency gate held)"
    );
    assert_eq!(
        tu_of(&app, shover),
        tu_before,
        "a rejected (non-adjacent) shove spends NO TU"
    );

    // Same-faction: an adjacent ALLY.
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let ally = shove_ganger(app.world_mut(), ground(6, 5), 0);
    app.update();
    let tu_before = tu_of(&app, shover);
    shove_and_settle(&mut app, shover, ally);
    assert_eq!(
        pos_of(&app, ally),
        Some(ground(6, 5)),
        "a same-faction ally is not shoved (the opposing-faction gate held)"
    );
    assert_eq!(
        tu_of(&app, shover),
        tu_before,
        "a rejected (ally) shove spends NO TU"
    );
}

// ── QA(9) — determinism ────────────────────────────────────────────────────────

/// QA(9): the same seed + same message order yields the IDENTICAL shove-off-a-ledge outcome
/// (landing storey + Hp loss) across two independent runs (the shove is RNG-free; the fall
/// draws deterministically from the seed).
#[test]
fn shove_outcomes_are_deterministic_under_same_seed() {
    let run = || -> (Option<CellLevel>, u16) {
        let mut app = shove_app();
        let mut surface = SurfaceGrid::new();
        surface.set_slab(upper(6, 5, 3), SlabState::Present);
        surface.set_slab(upper(7, 5, 1), SlabState::Present); // the fall lands on level 1
        app.insert_resource(surface);
        app.insert_resource(OccupancyGrid::new());
        let shover = shove_ganger(app.world_mut(), upper(5, 5, 3), 0);
        let target = shove_ganger(app.world_mut(), upper(6, 5, 3), 1);
        app.update();
        let before = hp_of(&app, target);
        shove_and_settle(&mut app, shover, target);
        (pos_of(&app, target), before - hp_of(&app, target))
    };
    assert_eq!(
        run(),
        run(),
        "same seed + same message order must yield identical shove/fall outcomes"
    );
}
