use crate::stability::types::{
    ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, SuppressionStability,
};

#[test]
fn output_newtypes_deref_to_inner() {
    assert_eq!((*ConeMult::new(0.7)).to_bits(), 0.7_f32.to_bits());
    assert_eq!((*RecoilGrowth::new(0.3)).to_bits(), 0.3_f32.to_bits());
    assert_eq!(
        (*EmplacementStability::new(8.0)).to_bits(),
        8.0_f32.to_bits()
    );
    assert_eq!((*EmplacementStability::none()).to_bits(), 0.0_f32.to_bits());
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
