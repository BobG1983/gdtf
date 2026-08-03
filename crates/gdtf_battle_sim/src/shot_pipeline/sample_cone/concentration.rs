//! How tightly shots cluster toward the aim axis.

use crate::{ganger::Shooting, tuning::ConcentrationCoeffs, weapon::Accuracy};

/// Concentration exponent used when sampling the cone.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConcentrationP(f32);

impl ConcentrationP {
    /// Build from a raw value.
    #[must_use]
    pub const fn new(p: f32) -> Self {
        Self(p)
    }
}

/// Concentration from shooting skill, weapon accuracy, and tuning coeffs.
#[must_use]
pub fn concentration_p(
    shooting: Shooting,
    accuracy: Accuracy,
    coeffs: ConcentrationCoeffs,
) -> ConcentrationP {
    let product = *shooting * *accuracy;
    let p = product.mul_add(*coeffs.scale, *coeffs.base);
    ConcentrationP::new(p)
}
