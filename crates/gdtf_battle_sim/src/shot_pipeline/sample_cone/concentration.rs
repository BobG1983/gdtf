use crate::{ganger::Shooting, tuning::ConcentrationCoeffs, weapon::Accuracy};

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConcentrationP(f32);

impl ConcentrationP {
            #[must_use]
    pub const fn new(p: f32) -> Self {
        Self(p)
    }
}

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
