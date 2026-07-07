//! Classifier-law unit tests (GTW-594 A1/C3): exactly one Active per frame, the
//! Isolate-wins precedence, and the per-mode band shapes.

use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};

use super::classify::{
    ContextDepth, IsolateView, StoreyTreatment, StoreyViewMode, storey_treatment,
};
use crate::{ActiveLevel, ViewMode};

/// Every `(view, isolate)` mode combination the sweep tests walk — the full input matrix.
fn all_modes() -> Vec<StoreyViewMode> {
    let isolates = [
        IsolateView::Off,
        IsolateView::On(ContextDepth::new(0)),
        IsolateView::On(ContextDepth::new(1)),
        IsolateView::On(ContextDepth::new(2)),
        IsolateView::On(ContextDepth::new(MAX_LEVELS)),
    ];
    let mut modes = Vec::new();
    for view in [ViewMode::DownToActive, ViewMode::FullView] {
        for isolate in isolates {
            modes.push(StoreyViewMode::new(view, isolate));
        }
    }
    modes
}

/// The A1 invariant law, first half: under EVERY mode × active-level combination,
/// EXACTLY ONE storey classifies [`StoreyTreatment::Active`] per frame — and it is the
/// active view storey itself (`Active ⇔ storey == active`).
#[test]
fn exactly_one_storey_classifies_active_per_frame() {
    for mode in all_modes() {
        for active_ix in 0..MAX_LEVELS {
            let active = ActiveLevel::new(Level::new(active_ix));
            let active_storeys: Vec<u8> = (0..MAX_LEVELS)
                .filter(|&storey| {
                    storey_treatment(Level::new(storey), active, mode) == StoreyTreatment::Active
                })
                .collect();
            assert_eq!(
                active_storeys,
                vec![active_ix],
                "exactly the active storey ({active_ix}) must classify Active under \
                 {mode:?} (A1: exactly-one-Active)",
            );
        }
    }
}

/// C3 — the precedence rule, pinned: while Isolate is ON the two-state [`ViewMode`] is
/// PREEMPTED — `DownToActive` and `FullView` classify every storey IDENTICALLY; while
/// Isolate is OFF the two modes genuinely differ (the above-active storeys).
#[test]
fn isolate_wins_over_the_two_state_view_mode() {
    let active = ActiveLevel::new(Level::new(3));
    let isolate = IsolateView::On(ContextDepth::new(1));
    for storey in 0..MAX_LEVELS {
        let level = Level::new(storey);
        assert_eq!(
            storey_treatment(
                level,
                active,
                StoreyViewMode::new(ViewMode::DownToActive, isolate)
            ),
            storey_treatment(
                level,
                active,
                StoreyViewMode::new(ViewMode::FullView, isolate)
            ),
            "with Isolate ON, DownToActive and FullView must classify storey {storey} \
             identically (Isolate WINS — GTW-594 C3)",
        );
    }
    // The control: with Isolate OFF the two modes differ (storey 5, above active 3, is
    // hidden in DownToActive and drawn in FullView) — so the equality above is not vacuous.
    let above = Level::new(5);
    assert_ne!(
        storey_treatment(
            above,
            active,
            StoreyViewMode::new(ViewMode::DownToActive, IsolateView::Off),
        ),
        storey_treatment(
            above,
            active,
            StoreyViewMode::new(ViewMode::FullView, IsolateView::Off),
        ),
        "with Isolate OFF the two-state modes must differ above the active storey",
    );
}

/// The Isolate band shape (C2): at active 3 with one onion storey below, storeys 2 and 3
/// draw (context at depth 1, then Active) and EVERYTHING else — the deeper stack AND
/// everything above — is hidden. The band FLOOR is `active - depth`, not the ground.
#[test]
fn isolate_band_is_the_active_storey_plus_its_onion() {
    let active = ActiveLevel::new(Level::new(3));
    let mode = StoreyViewMode::new(
        ViewMode::DownToActive,
        IsolateView::On(ContextDepth::new(1)),
    );
    let expect = |storey: u8| storey_treatment(Level::new(storey), active, mode);
    assert_eq!(expect(3), StoreyTreatment::Active, "the active storey");
    assert_eq!(
        expect(2),
        StoreyTreatment::ContextBelow(ContextDepth::new(1)),
        "one below = the onion context at depth 1",
    );
    for hidden in [0_u8, 1, 4, 5, 6, 7] {
        assert_eq!(
            expect(hidden),
            StoreyTreatment::Hidden,
            "storey {hidden} is outside the isolate band (floor = active - onion)",
        );
    }
    // Isolate at the ground floor: nothing below exists to onion — only the active draws.
    let ground = ActiveLevel::new(Level::new(0));
    assert_eq!(
        storey_treatment(Level::new(0), ground, mode),
        StoreyTreatment::Active,
        "isolate at ground: the ground storey is Active",
    );
    assert_eq!(
        storey_treatment(Level::new(1), ground, mode),
        StoreyTreatment::Hidden,
        "isolate at ground: the storey above stays hidden",
    );
}

/// The two-state band shapes are UNCHANGED with Isolate off (A3 — the pre-GTW-594
/// behaviour): `DownToActive` classifies `0..=active` drawn (context below at its true
/// depth, above hidden); `FullView` classifies EVERY storey drawn.
#[test]
fn two_state_modes_are_unchanged_with_isolate_off() {
    let active = ActiveLevel::new(Level::new(2));
    let down = StoreyViewMode::new(ViewMode::DownToActive, IsolateView::Off);
    let full = StoreyViewMode::new(ViewMode::FullView, IsolateView::Off);

    assert_eq!(
        storey_treatment(Level::new(0), active, down),
        StoreyTreatment::ContextBelow(ContextDepth::new(2)),
        "DownToActive: two below the active is context at depth 2 (the true distance)",
    );
    assert_eq!(
        storey_treatment(Level::new(3), active, down),
        StoreyTreatment::Hidden,
        "DownToActive: above the active is the canon hard cut",
    );
    for storey in 0..MAX_LEVELS {
        assert_ne!(
            storey_treatment(Level::new(storey), active, full),
            StoreyTreatment::Hidden,
            "FullView draws every storey (the GTW-521 whole stack) — storey {storey}",
        );
    }
    assert_eq!(
        storey_treatment(Level::new(4), active, full),
        StoreyTreatment::ContextBelow(ContextDepth::new(2)),
        "FullView: an above-active storey classifies as context at its distance (the \
         documented legacy exception — GTW-594 C4 defers any distinct treatment)",
    );
}
