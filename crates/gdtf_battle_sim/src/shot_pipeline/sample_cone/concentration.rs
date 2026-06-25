//! The §1b **concentration exponent** — the [`ConcentrationP`] newtype and
//! [`concentration_p`], the power-law exponent of the biased radius `rand^p`.

use crate::{ganger::Shooting, tuning::ConcentrationCoeffs, weapon::Accuracy};

/// The **concentration exponent `p`** of the §1b power-law radius `rand^p`
/// (resolution.md §1b). `p ≈ 1` scatters the in-cone draw evenly out to the cone
/// edge; a larger `p` clusters it near dead-center. It rises with accuracy
/// (`Shooting × weapon.accuracy`).
///
/// The named exponent newtype (no-bare-types: the power-law exponent is a domain
/// value, not a bare `f32`), returned by [`concentration_p`] and fed to
/// [`crate::sample_cone::sample_cone_vector`]. Dimensionless — **zero pixels**.
/// Private inner + derived [`Deref`](bevy::prelude::Deref).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConcentrationP(f32);

impl ConcentrationP {
    /// Build a concentration exponent from its magnitude (dimensionless; the
    /// power-law exponent of the §1b biased radius).
    #[must_use]
    pub const fn new(p: f32) -> Self {
        Self(p)
    }
}

/// The §1b concentration exponent `p = base + scale × (Shooting × weapon.accuracy)`
/// (resolution.md §1b: `p = concentration, rising with accuracy = Shooting ×
/// weapon.accuracy`; "What's pure math vs sim" line 150: `p =
/// concentration_p(Shooting, weapon.accuracy)`, concentration toward dead-center
/// rising with it).
///
/// The accuracy product `Shooting × weapon.accuracy` is mapped through the E2.1
/// [`ConcentrationCoeffs`] (a `base` exponent at zero accuracy plus a per-accuracy
/// `scale`) — so the curve is a data edit, not a code change (AC #6: coefficients
/// from tuning, none hardcoded). With a positive `scale`, `p` rises monotonically
/// with accuracy (AC #5): higher Shooting or a more accurate weapon yields a larger
/// `p`, which [`crate::sample_cone::sample_cone_vector`] uses to cluster shots nearer
/// the axis. The weapon term [`Accuracy`] may exceed 1.0 (resolution.md §1b).
///
/// Dimensionless — **zero pixels**. Returns the named [`ConcentrationP`]
/// (no-bare-types).
#[must_use]
pub fn concentration_p(
    shooting: Shooting,
    accuracy: Accuracy,
    coeffs: ConcentrationCoeffs,
) -> ConcentrationP {
    // p = base + scale × (Shooting × accuracy). The accuracy product is the §1b
    // `Shooting × weapon.accuracy`; the base/scale coefficients are tuning data.
    let product = *shooting * *accuracy;
    let p = product.mul_add(*coeffs.scale, *coeffs.base);
    ConcentrationP::new(p)
}
