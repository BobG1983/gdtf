//! Posture & identity state — the [`Stance`]/[`StanceKind`] posture, the
//! [`Aiming`] aim-mode flag, and the [`Faction`] gang identity.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// The three postures a ganger can hold.
///
/// Posture reshapes the stability score and the clearance silhouette
/// (resolution.md §1: "prone 40 / kneel 25 / stand 10"; §4 stance reshapes the
/// bands). `Crouching` is the doc's "kneel" posture. Named domain kinds, not a
/// bare integer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum StanceKind {
    /// Upright — full stability gate, full silhouette (the doc's "stand").
    #[default]
    Standing,
    /// Kneeling — compressed silhouette, steadier than standing (the doc's "kneel").
    Crouching,
    /// Flat — lowest silhouette, steadiest, but cannot clear even LOW cover.
    Prone,
}

/// A ganger's stance — which [`StanceKind`] posture it currently holds.
///
/// A distinct component so a stability / clearance system can query `&Stance`
/// alone. Defaults to [`StanceKind::Standing`] (the structural spawn posture, not
/// a tunable). `#[serde(transparent)]` lets an authored stance parse as the bare
/// [`StanceKind`] variant.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Stance(pub(super) StanceKind);

impl Stance {
    /// Build a stance from the [`StanceKind`] posture the ganger holds.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// a `Stance` from an authored posture without reaching the private field.
    #[must_use]
    pub const fn new(posture: StanceKind) -> Self {
        Self(posture)
    }
}

/// Whether a ganger is **aiming** (aimed shot) rather than hip-firing.
///
/// The Aim-Mode axis from resolution.md §1a: aiming narrows the dispersion cone
/// (×0.6) at a TU premium, hip-fired does not. A distinct component so a shot /
/// HUD system can query `&Aiming` alone. Defaults to `false` (hip-fired — a fresh
/// ganger is not aiming; a structural default, not a balance value).
/// `#[serde(transparent)]` lets an authored aim-mode parse as a bare boolean.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Aiming(pub(super) bool);

impl Aiming {
    /// Build an aim-mode flag — `true` for aimed fire, `false` for hip-fired.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// an `Aiming` from an authored value without reaching the private field.
    #[must_use]
    pub const fn new(aiming: bool) -> Self {
        Self(aiming)
    }
}

/// A gang (faction) identity — which side a ganger fights for.
///
/// Wraps a small gang index (glossary: a *Gang* is a faction / the player's
/// roster as a unit). The shooter/target faction decides friend from foe for
/// targeting and the friendly-fire path (resolution.md §2: "any other actor in
/// the path — including your own gang"). A distinct component so a targeting
/// system can query `&Faction` alone. Defaults to gang `0`.
/// `#[serde(transparent)]` lets an authored gang index parse as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Faction(pub(super) u8);

impl Faction {
    /// Build a faction (gang) identity from its small gang index.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// a `Faction` from an authored gang index without reaching the private field.
    #[must_use]
    pub const fn new(gang: u8) -> Self {
        Self(gang)
    }
}
