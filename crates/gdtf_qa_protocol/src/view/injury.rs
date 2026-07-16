//! The injury-summary DTOs — the wire mirror of a ganger's inflicted-injury ledger
//! (GTW-734).
//!
//! A curated summary of the sim's `InflictedInjuries` ledger: the total count plus a
//! per-injury entry (name / struck part / severity). The frozen `InjuryEffect`
//! magnitudes (the balance data, some `f32`) are NOT surfaced — a QA client reads WHAT
//! was inflicted, not the internal effect coefficients.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// A rolled wound **severity** bucket — the wire mirror of the sim `Severity`.
///
/// The five buckets, ascending (graze → death): [`None`](Self::None) (HP-only graze,
/// no Wound spent), [`Minor`](Self::Minor), [`Major`](Self::Major),
/// [`Critical`](Self::Critical), [`Fatal`](Self::Fatal) (empties the Wounds pool). An
/// independent serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeverityNet {
    /// A graze — HP loss only, no Wound spent.
    None,
    /// A minor wound.
    Minor,
    /// A major wound.
    Major,
    /// A critical wound.
    Critical,
    /// A fatal hit — death in battle.
    Fatal,
}

/// A struck **body part** — the wire mirror of the sim `BodyPart`.
///
/// The six canonical parts; an independent serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyPartNet {
    /// The head.
    Head,
    /// The torso.
    Torso,
    /// The left arm.
    LeftArm,
    /// The right arm.
    RightArm,
    /// The left leg.
    LeftLeg,
    /// The right leg.
    RightLeg,
}

/// An injury's display **name** — the wire mirror of the sim `InjuryName` (`String`).
///
/// A name newtype (no-bare-types), serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InjuryNameNet(String);

impl InjuryNameNet {
    /// Build an injury name from its display string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// One durable injury on a ganger's ledger — its name, struck part, and severity.
///
/// A curated entry (a subset of the sim's `GainedInjury`, dropping the frozen effect
/// magnitudes). Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InjuryEntryNet {
    /// The injury's display name.
    pub name:     InjuryNameNet,
    /// The struck body part.
    pub part:     BodyPartNet,
    /// The rolled severity bucket.
    pub severity: SeverityNet,
}

impl InjuryEntryNet {
    /// Build a ledger entry from its name, part, and severity.
    #[must_use]
    pub const fn new(name: InjuryNameNet, part: BodyPartNet, severity: SeverityNet) -> Self {
        Self {
            name,
            part,
            severity,
        }
    }
}

/// A ganger's injury **summary** — the durable ledger as a wire list.
///
/// The mirror of the sim `InflictedInjuries` ledger: the ordered [`entries`](Self::entries)
/// a ganger has accrued. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InjurySummaryNet {
    /// The durable injuries the ganger carries, in accrual order.
    pub entries: Vec<InjuryEntryNet>,
}

impl InjurySummaryNet {
    /// Build an injury summary from its ledger entries.
    #[must_use]
    pub const fn new(entries: Vec<InjuryEntryNet>) -> Self {
        Self { entries }
    }
}
