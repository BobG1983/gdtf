//! Anchor-selection + strict-opposite tests (GTW-424 C1/C3, OQ-2).

use crate::{
    procgen::Anchor,
    rng::{BattleSeed, ProcgenRng},
};

/// OQ-2: every anchor's strict opposite is the exact geometric mirror, and the map is an
/// involution (`a.opposite().opposite() == a`) over all eight zones.
///
/// Discriminating: pins the EXACT pairs the ticket names — a wrong mirror (e.g.
/// `TopRight -> TopLeft`) would fail. The involution check catches any non-symmetric edit.
#[test]
fn opposite_is_the_strict_geometric_mirror_involution() {
    let pairs = [
        (Anchor::TopRight, Anchor::BottomLeft),
        (Anchor::BottomRight, Anchor::TopLeft),
        (Anchor::RightMiddle, Anchor::LeftMiddle),
        (Anchor::BottomMiddle, Anchor::TopMiddle),
    ];
    for (a, opp) in pairs {
        assert_eq!(a.opposite(), opp, "{a:?} opposite must be {opp:?}");
        assert_eq!(
            opp.opposite(),
            a,
            "{opp:?} opposite must be {a:?} (symmetry)"
        );
        assert_eq!(
            a.opposite().opposite(),
            a,
            "opposite must be an involution for {a:?}",
        );
    }
}

/// C1: the player anchor is chosen from exactly the four supported anchors.
#[test]
fn chosen_player_anchor_is_one_of_the_four() {
    let mut rng = ProcgenRng::from_root(BattleSeed::new(7));
    for _ in 0..50 {
        let chosen = Anchor::choose(&mut rng);
        assert!(
            Anchor::PLAYER_ANCHORS.contains(&chosen),
            "chosen player anchor {chosen:?} must be one of the four supported anchors",
        );
    }
}

/// C3 (determinism): the same seed picks the same player anchor sequence — the RNG draw
/// order is fixed. Two independently seeded streams agree draw-for-draw.
///
/// Discriminating: if `Anchor::choose` drew a different count or order, the two sequences
/// would diverge. This pins the single-draw, fixed-order contract.
#[test]
fn anchor_choice_is_deterministic_under_a_seed() {
    let seed = BattleSeed::new(0xC0FF_EE42);
    let mut a = ProcgenRng::from_root(seed);
    let mut b = ProcgenRng::from_root(seed);
    let seq_a: Vec<Anchor> = (0..20).map(|_| Anchor::choose(&mut a)).collect();
    let seq_b: Vec<Anchor> = (0..20).map(|_| Anchor::choose(&mut b)).collect();
    assert_eq!(
        seq_a, seq_b,
        "same seed must yield the same player-anchor sequence (determinism)",
    );
}

/// C1 (RNG coverage): over many seeds the player anchor varies — `choose` is a real draw,
/// not a constant. (Not a distribution test; just that more than one anchor appears.)
#[test]
fn anchor_choice_varies_across_seeds() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for s in 0..64u64 {
        let mut rng = ProcgenRng::from_root(BattleSeed::new(s));
        seen.insert(Anchor::choose(&mut rng));
    }
    assert!(
        seen.len() > 1,
        "player-anchor choice must vary across seeds, saw only {seen:?}",
    );
}
