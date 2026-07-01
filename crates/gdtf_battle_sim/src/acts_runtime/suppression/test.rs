//! In-crate unit tests for the suppression module's PURE pieces (GTW-526) — the
//! band→stance mapping and the geometry helpers. The SYSTEM-level end-to-end proofs
//! (producer / determinism / clear cadence / auto-stance / idempotent refresh) live in
//! the integration test `crates/gdtf_battle_sim/tests/gtw526_suppression_core.rs`, driven
//! on the real `BattleSimPlugin` runtime (the gtw468 reaction harness idiom).

use super::stance::stance_for_cover_band;
use crate::{cover::HeightBand, ganger::StanceKind};

/// C5 — the pure band→stance rule: Low → Prone, Mid → Crouching, High → Crouching (the
/// stairs-as-High mapping folds into the High arm). Exhaustive over the three bands.
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
