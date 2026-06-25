//! The [`InflictedWound`] record value + the [`InflictedWounds`] component list.
//! See the module docs (`super`) for the wound-to-pool mapping (AC3) and the
//! presentation-record contract.

use bevy::prelude::{Component, Deref};

use crate::{armor::BodyPart, severity::Severity};

/// One **inflicted wound** a ganger has taken — its rolled severity TIER and the
/// struck BODY PART (`docs/combat/resolution.md` §4 / §6).
///
/// The atomic record the GTW-278 wound-name panel renders (e.g. "Minor — Left
/// Arm"): the [`tier`](InflictedWound::tier) is the §6 severity bucket the
/// resolution rolled, the [`location`](InflictedWound::location) is the §4 struck
/// part. Both fields are the EXISTING sim newtypes — the [`Severity`] tier ladder
/// and the [`BodyPart`] location vocabulary — so this record introduces no new bare
/// enum / string (no-bare-types: its fields are already typed domain values).
///
/// A `Copy` value object (both fields are `Copy`), built only at the wound-infliction
/// site ([`apply_hit`](crate::apply_hit::apply_hit)) and pushed onto the target's
/// [`InflictedWounds`]; the sim never reads it back for combat math.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InflictedWound {
    /// The rolled wound-severity tier (`docs/combat/resolution.md` §6) — the
    /// non-`None` bucket this hit landed in (a graze records no wound, so this is
    /// never [`Severity::None`] in a recorded entry).
    pub tier:     Severity,
    /// The struck body part (`docs/combat/resolution.md` §4) — the §4 weighted
    /// location roll's outcome for the hit that inflicted this wound.
    pub location: BodyPart,
}

impl InflictedWound {
    /// Build an inflicted-wound record from its rolled tier and struck location.
    ///
    /// The public constructor (house style) so the E3.6 application path can record
    /// a wound without reaching the fields positionally.
    #[must_use]
    pub const fn new(tier: Severity, location: BodyPart) -> Self {
        Self { tier, location }
    }
}

/// A ganger's **inflicted-wound list** — the queryable record of every wound it has
/// taken, in infliction order (GTW-279).
///
/// A component on the ganger entity holding the running list of [`InflictedWound`]s.
/// Seeded **empty** on every spawned ganger (the [`Default`]) in
/// [`setup_battle`](crate::situation::setup_battle), alongside the existing vitals,
/// and appended at the E3.6 wound-application fold each time a wound registers (the
/// SAME place the [`Wounds`](crate::ganger::Wounds) pool is spent — see the module
/// docs for the per-tier mapping). The list is **additive**: it only grows, in the
/// order wounds were taken, so the view can render the full injury history.
///
/// A named newtype over [`Vec<InflictedWound>`] (no-bare-types: the wound list is a
/// domain value, not a bare `Vec`) with a **private** inner + derived [`Deref`]
/// (house style — read the list by `Deref` to `&[InflictedWound]`; the only mutation
/// is the sim's own [`record`](InflictedWounds::record) append). A `#[derive(Component)]`
/// so the GTW-278 status / hover panels can query `&InflictedWounds` off the ganger.
/// It is a presentation record: the sim NEVER reads it back for combat math.
#[derive(Deref, Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct InflictedWounds(Vec<InflictedWound>);

impl InflictedWounds {
    /// Build an inflicted-wound list from its records.
    ///
    /// The public constructor (house style) so tests / setup can build a populated
    /// list without reaching the private field. A freshly spawned ganger uses the
    /// empty [`Default`] instead.
    #[must_use]
    pub const fn new(wounds: Vec<InflictedWound>) -> Self {
        Self(wounds)
    }

    /// Append one [`InflictedWound`] to the list, preserving infliction order.
    ///
    /// The sole mutator — the E3.6 wound-application fold
    /// ([`apply_hit`](crate::apply_hit::apply_hit)) calls this once per registered
    /// (non-graze) wound, in the same place the [`Wounds`](crate::ganger::Wounds)
    /// pool is spent, so the list stays consistent with the pool. The sim never
    /// removes or reorders entries.
    pub fn record(&mut self, wound: InflictedWound) {
        self.0.push(wound);
    }
}
