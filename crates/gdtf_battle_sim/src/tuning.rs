//! Combat tuning data — the single home for every balance coefficient.
//!
//! The equation *forms* live in code (`docs/combat/resolution.md`); every
//! *coefficient* is a default here, so balancing is a data edit, not a code
//! change ("Coefficients live in the combat-tuning data", resolution.md §"What's
//! pure math vs sim"). [`CombatTuning`] is a Bevy [`Resource`] that
//! deserializes from a `.ron` file, and **no numeric tuning literal lives
//! anywhere outside this module** — the [`crate::metric::MAX_LEVELS`]
//! coordinate-system constant is the only other named number.
//!
//! Every numeric leaf is a named newtype (no bare `f32`/`u16` field), per the
//! no-bare-types rule: each carries a derived [`Deref`] to its inner value and
//! `#[serde(transparent)]` so it round-trips as a bare RON scalar.

use bevy::prelude::{Deref, Resource};
use serde::Deserialize;

/// A projectile clearance band edge, as a **level-fraction** — a dimensionless
/// fraction of one level's height (`z ∈ [0,1)` within a storey).
///
/// One newtype shared by **both** band edges of [`ProjectileBandEdges`]: the two
/// edges are the same *kind* of value (a level-fraction clearance threshold),
/// distinguished by their field. Because they are fractions of a storey they
/// re-scale with the cubic voxel and carry no pixel (`docs/combat/battle-space.md`
/// §"Banding"). `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BandEdge(f32);

/// `j` — the penetrating-damage scale: how hard pen damage pushes severity up
/// (resolution.md §6 severity score).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct PenDamageScale(f32);

/// `k` — Toughness mitigation: how much the defender's Toughness subtracts from
/// the severity score (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ToughnessMitigation(f32);

/// `I` — shooter-luck scale: the shooter's Luck adds to the severity score,
/// nudging toward nastier wounds (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ShooterLuckScale(f32);

/// `L` — defender-luck spread cap: the defender's Luck shrinks the random spread
/// (`R − L × Luck_defender`), capping the bad tail (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct DefenderLuckSpreadCap(f32);

/// `R` — the one-sided random spread before the defender's Luck trims it
/// (resolution.md §6: `roll(0..R_eff)`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RandomSpread(f32);

/// `R_min` — the floor on the trimmed spread (`max(R_min, …)`), so the
/// defender's Luck can never erase the random term entirely (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RandomSpreadMin(f32);

/// The relative weight of one body part in the §4 `roll_body_part` weighted roll.
///
/// One newtype reused by all six fields of [`BodyPartWeights`]: each part's
/// weight is the same *kind* of value (a relative pick weight), distinguished by
/// its field. A small non-negative integer summed into a weighted pick;
/// `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BodyPartWeight(u16);

/// The projectile clearance band edges, as **level-fractions** within one
/// level's height.
///
/// The march bands each crossed cell LOW / MID / HIGH by the round's continuous
/// `z` within the crossed level and compares it to the occupant's band
/// (resolution.md §2). The edges are dimensionless fractions of a storey
/// (defaults ≈ ⅓ and ⅔ of a level), so they re-scale with the cubic voxel and
/// carry no pixel (`docs/combat/battle-space.md` §"Banding"). `low_mid` is the
/// LOW→MID edge, `mid_high` the MID→HIGH edge.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ProjectileBandEdges {
    /// The LOW→MID clearance edge (level-fraction). Default ≈ ⅓ of a level.
    pub low_mid:  BandEdge,
    /// The MID→HIGH clearance edge (level-fraction). Default ≈ ⅔ of a level.
    pub mid_high: BandEdge,
}

impl Default for ProjectileBandEdges {
    fn default() -> Self {
        // Tunable level-fraction defaults (≈ ⅓ and ⅔ of a level) from
        // docs/combat/battle-space.md §"Banding". These are balance data, not a
        // coordinate-system fact — value-agnostic tests only.
        Self {
            low_mid:  BandEdge(0.33),
            mid_high: BandEdge(0.67),
        }
    }
}

/// The wound-severity scaling scalars from resolution.md §6.
///
/// They scale the severity score (form in resolution.md §6): `pen_damage_scale`
/// times penetrating damage, minus `toughness_mitigation` times Toughness, plus
/// the part modifier, the weapon fatal-bias, and `shooter_luck_scale` times the
/// shooter's Luck, plus a one-sided random draw over `0..R_eff` where
/// `R_eff = max(random_spread_min, random_spread - defender_luck_spread_cap *
/// Luck_defender)`. Each field is its own named newtype over the doc symbol
/// (`j`/`k`/`I`/`L`/`R`/`R_min`) so distinct concepts stay distinct types;
/// magnitudes are tunable defaults.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityScaling {
    /// `j` — penetrating-damage scale: how hard pen damage pushes severity up.
    pub pen_damage_scale:         PenDamageScale,
    /// `k` — Toughness mitigation: how much the defender's Toughness subtracts.
    pub toughness_mitigation:     ToughnessMitigation,
    /// `I` — shooter-luck scale: the shooter's Luck adds (nastier wounds).
    pub shooter_luck_scale:       ShooterLuckScale,
    /// `L` — defender-luck spread cap: the defender's Luck shrinks the random
    /// spread (`R − L × Luck_defender`).
    pub defender_luck_spread_cap: DefenderLuckSpreadCap,
    /// `R` — the one-sided random spread before the defender's Luck trims it.
    pub random_spread:            RandomSpread,
    /// `R_min` — the floor on the trimmed spread (`max(R_min, …)`), so Luck can
    /// never erase the random term entirely.
    pub random_spread_min:        RandomSpreadMin,
}

impl Default for SeverityScaling {
    fn default() -> Self {
        // TUNABLE placeholder magnitudes for the resolution.md §6 symbols — the
        // forms are fixed, these numbers are balance data (de-brittled: tests
        // exercise the serde mechanism, not these shipped values).
        Self {
            pen_damage_scale:         PenDamageScale(1.0),
            toughness_mitigation:     ToughnessMitigation(1.0),
            shooter_luck_scale:       ShooterLuckScale(1.0),
            defender_luck_spread_cap: DefenderLuckSpreadCap(1.0),
            random_spread:            RandomSpread(10.0),
            random_spread_min:        RandomSpreadMin(1.0),
        }
    }
}

/// The body-part hit-location weights — the relative weight of each of the six
/// parts in the §4 `roll_body_part` weighted roll.
///
/// Head is rare, torso the bulk (resolution.md §4: Head 6 / Torso 40 / each Arm
/// 15 / each Leg 12). Each field is a [`BodyPartWeight`]; the magnitudes are
/// tunable balance data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BodyPartWeights {
    /// Head weight — rare (doc default 6).
    pub head:      BodyPartWeight,
    /// Torso weight — the bulk of hits (doc default 40).
    pub torso:     BodyPartWeight,
    /// Left-arm weight (doc default 15).
    pub left_arm:  BodyPartWeight,
    /// Right-arm weight (doc default 15).
    pub right_arm: BodyPartWeight,
    /// Left-leg weight (doc default 12).
    pub left_leg:  BodyPartWeight,
    /// Right-leg weight (doc default 12).
    pub right_leg: BodyPartWeight,
}

impl Default for BodyPartWeights {
    fn default() -> Self {
        // Defaults from docs/combat/resolution.md §4.
        Self {
            head:      BodyPartWeight(6),
            torso:     BodyPartWeight(40),
            left_arm:  BodyPartWeight(15),
            right_arm: BodyPartWeight(15),
            left_leg:  BodyPartWeight(12),
            right_leg: BodyPartWeight(12),
        }
    }
}

/// The combat tuning resource — every balance coefficient the sim marches with.
///
/// A Bevy [`Resource`] deserializable from a `.ron` file (the tuning store is a
/// serde-loaded resource, resolution.md §"Coefficients live in the
/// combat-tuning data"). Holds the clearance band edges, the §6 severity
/// scaling, and the body-part weights; more sub-fields land as the E1 systems
/// do. Defaults carry the doc values, but they are **tunable** — a data file
/// overrides any of them.
#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize)]
pub struct CombatTuning {
    /// Projectile clearance band edges (the LOW/MID/HIGH thresholds).
    pub projectile_band_edges: ProjectileBandEdges,
    /// The resolution.md §6 wound-severity scaling scalars.
    pub severity_scaling:      SeverityScaling,
    /// The §4 body-part hit-location weights.
    pub body_part_weights:     BodyPartWeights,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each tuning newtype wraps the right inner type and its derived [`Deref`]
    /// reaches that inner value (C9/C10/C11 mandate a derived `Deref` on every
    /// tuning newtype; this exercises that surface so dropping the derive would
    /// fail a test).
    ///
    /// Built from **arbitrary** literals, never the shipped/default magnitudes:
    /// this pins the Deref *target type and mechanism*, not a balance value, so
    /// it stays non-brittle against a tuning edit. The seven `f32` newtypes are
    /// compared by bit pattern (the literals are exactly representable, so this
    /// is an exact integer equality — no `float_cmp` lint, no epsilon).
    #[test]
    fn tuning_newtypes_wrap_inner_and_deref() {
        // Each f32 newtype: deref reaches the inner f32 (bit-exact arbitrary
        // value, not the default).
        assert_eq!((*BandEdge(5.0)).to_bits(), 5.0_f32.to_bits());
        assert_eq!((*PenDamageScale(2.5)).to_bits(), 2.5_f32.to_bits());
        assert_eq!((*ToughnessMitigation(3.5)).to_bits(), 3.5_f32.to_bits());
        assert_eq!((*ShooterLuckScale(4.5)).to_bits(), 4.5_f32.to_bits());
        assert_eq!((*DefenderLuckSpreadCap(6.5)).to_bits(), 6.5_f32.to_bits());
        assert_eq!((*RandomSpread(7.5)).to_bits(), 7.5_f32.to_bits());
        assert_eq!((*RandomSpreadMin(8.5)).to_bits(), 8.5_f32.to_bits());
        // The u16 newtype: deref reaches the inner u16 (arbitrary value).
        assert_eq!(*BodyPartWeight(3), 3u16);
    }

    /// The shipped `assets/combat/tuning.ron` deserializes into a
    /// [`CombatTuning`] on the **real** path (the same file the data-driven
    /// tuning store loads).
    ///
    /// Deliberately value-agnostic: the tuning magnitudes are the **tunable**
    /// balance data, so this pins only that the shipped file parses into the
    /// type — never a specific band edge, severity scalar, or part weight
    /// (asserting a magnitude would be brittle against a balance edit). The
    /// metric-const pin lives in [`crate::metric`] (a coordinate-system
    /// definition, not a tunable).
    #[test]
    fn shipped_tuning_ron_deserializes() {
        const SHIPPED_TUNING_RON: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/combat/tuning.ron"
        ));

        let parsed = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON);
        assert!(
            parsed.is_ok(),
            "shipped assets/combat/tuning.ron must deserialize into CombatTuning: {parsed:?}",
        );
    }
}
