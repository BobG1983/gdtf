//! The output-newtype Deref mechanism (no-bare-types rule 3).

use crate::stability::types::{
    ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, SuppressionStability,
};

/// The output newtypes' derived [`Deref`] reaches their inner value, and the
/// two outputs are DISTINCT types (no-bare-types rule 3). Built from arbitrary
/// literals — pins the Deref mechanism + target type, not a magnitude.
#[test]
fn output_newtypes_deref_to_inner() {
    assert_eq!((*ConeMult::new(0.7)).to_bits(), 0.7_f32.to_bits());
    assert_eq!((*RecoilGrowth::new(0.3)).to_bits(), 0.3_f32.to_bits());
    assert_eq!(
        (*EmplacementStability::new(8.0)).to_bits(),
        8.0_f32.to_bits()
    );
    assert_eq!((*EmplacementStability::none()).to_bits(), 0.0_f32.to_bits());
    // GTW-526: the suppression term derefs to its (negative-in-practice) inner, and its
    // identity is 0.0 (the byte-identity term for an un-suppressed shooter).
    assert_eq!(
        (*SuppressionStability::new(-40.0)).to_bits(),
        (-40.0_f32).to_bits()
    );
    assert_eq!((*SuppressionStability::none()).to_bits(), 0.0_f32.to_bits());
    assert_eq!(
        (*StabilityScore::clamped(50.0)).to_bits(),
        50.0_f32.to_bits()
    );
}
