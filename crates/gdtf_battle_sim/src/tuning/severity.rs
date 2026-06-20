//! The resolution.md §6 wound-severity scaling scalars + the bucket-edge ladder.

use bevy::prelude::Deref;
use serde::Deserialize;

/// `j` — the penetrating-damage scale: how hard pen damage pushes severity up
/// (resolution.md §6 severity score).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct PenDamageScale(f32);

impl PenDamageScale {
    /// Build a penetrating-damage scale from its coefficient magnitude — for tests
    /// and programmatic tuning edits; shipped values come from the `.ron` via the
    /// derived [`Deserialize`]. Private inner (house style).
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// `k` — Toughness mitigation: how much the defender's Toughness subtracts from
/// the severity score (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ToughnessMitigation(f32);

impl ToughnessMitigation {
    /// Build a Toughness-mitigation coefficient from its magnitude — for tests and
    /// programmatic tuning edits; shipped values come from the `.ron` via the derived
    /// [`Deserialize`]. Private inner (house style).
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// `I` — shooter-luck scale: the shooter's Luck adds to the severity score,
/// nudging toward nastier wounds (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ShooterLuckScale(f32);

impl ShooterLuckScale {
    /// Build a shooter-luck scale from its coefficient magnitude — for tests and
    /// programmatic tuning edits; shipped values come from the `.ron` via the derived
    /// [`Deserialize`]. Private inner (house style).
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// `L` — defender-luck scale: how far the defender's Luck pushes the severity
/// roll's **lower bound below 0** (resolution.md §6, floor-extend form
/// `roll(−L × Luck_defender .. R)`).
///
/// The defender's Luck **extends the roll's floor downward** — a chance to shrug
/// the hit off — while the ceiling stays `R`, so a genuinely bad roll is always
/// still possible (variance grows with the defender's Luck). This is **not** a
/// spread cap (the old `R_eff = max(R_min, R − L × Luck_defender)` form is gone):
/// the lower bound *moves*, the spread is not merely shrunk.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct DefenderLuckScale(f32);

impl DefenderLuckScale {
    /// Build a defender-luck scale from its coefficient magnitude — for tests and
    /// programmatic tuning edits; shipped values come from the `.ron` via the derived
    /// [`Deserialize`]. Private inner (house style).
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// `R` — the **upper bound** (ceiling) of the severity roll's random term
/// (resolution.md §6: `roll(−L × Luck_defender .. R)`).
///
/// The ceiling is fixed at `R` regardless of either ganger's Luck — only the
/// floor moves (down with the defender's Luck via [`DefenderLuckScale`]), so the
/// worst-case roll is unchanged.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RandomSpread(f32);

impl RandomSpread {
    /// Build a random-spread ceiling from its magnitude — for tests and programmatic
    /// tuning edits; shipped values come from the `.ron` via the derived
    /// [`Deserialize`]. Private inner (house style).
    #[must_use]
    pub const fn new(spread: f32) -> Self {
        Self(spread)
    }
}

/// A single **severity-bucket edge** — one ascending threshold on the §6
/// severity score (resolution.md §6: `< e0 → None`, `< e1 → Minor`, …,
/// `≥ e3 → Fatal`).
///
/// One newtype shared by all four edges of [`SeverityEdges`]: each edge is the
/// same *kind* of value (a score threshold separating two adjacent severity
/// buckets), distinguished by its field. A dimensionless score threshold;
/// `#[serde(transparent)]` lets it parse a bare RON scalar. Magnitudes are
/// tunable balance data — never pinned by a value test.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SeverityEdge(f32);

impl SeverityEdge {
    /// Build a severity-bucket edge from its score-threshold magnitude (TBD
    /// tuning) — for tests and programmatic tuning edits; shipped values come
    /// from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(edge: f32) -> Self {
        Self(edge)
    }
}

/// The four ascending **severity-bucket edges** that tier the §6 severity score
/// into a [`crate::severity::Severity`] (resolution.md §6).
///
/// The score is bucketed `< e0 → None`, `< e1 → Minor`, `< e2 → Major`,
/// `< e3 → Critical`, `≥ e3 → Fatal`. The edges must form an ascending ladder
/// (`e0 < e1 < e2 < e3`) for the buckets to be monotone in the score; the
/// magnitudes are tunable balance data — never pinned by a value test.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityEdges {
    /// `e0` — the None→Minor edge (below it is a graze, no wound).
    pub e0: SeverityEdge,
    /// `e1` — the Minor→Major edge.
    pub e1: SeverityEdge,
    /// `e2` — the Major→Critical edge.
    pub e2: SeverityEdge,
    /// `e3` — the Critical→Fatal edge (at or above it is a Fatal hit).
    pub e3: SeverityEdge,
}

impl Default for SeverityEdges {
    fn default() -> Self {
        // An ascending placeholder ladder over the §6 severity score — only the
        // ordering (e0 < e1 < e2 < e3) is a fixed mechanism; the magnitudes are
        // TUNABLE balance data, value-agnostic tests only.
        Self {
            e0: SeverityEdge::new(1.0),
            e1: SeverityEdge::new(5.0),
            e2: SeverityEdge::new(10.0),
            e3: SeverityEdge::new(15.0),
        }
    }
}

/// The wound-severity scaling scalars from resolution.md §6 — the **floor-extend**
/// form (resolved 2026-06-15).
///
/// They scale the severity score (form in resolution.md §6): `pen_damage_scale`
/// times penetrating damage, minus `toughness_mitigation` times Toughness, plus
/// the part modifier, the weapon fatal-bias, and `shooter_luck_scale` times the
/// shooter's Luck, plus a random draw over `roll(−L × Luck_defender .. R)` — the
/// defender's Luck (`defender_luck_scale`, `L`) extends the roll's **floor**
/// downward (a shrug-off chance) while the **ceiling stays `R`** (`random_spread`).
/// There is **no** `R_eff` / `R_min` (the old spread-cap form is gone). The
/// nested [`SeverityEdges`] ride along here so the §6 score and its bucket ladder
/// share one tuning struct (the severity roll takes only `&SeverityScaling`).
/// Each field is its own named newtype over the doc symbol (`j`/`k`/`I`/`L`/`R`)
/// so distinct concepts stay distinct types; magnitudes are tunable defaults.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityScaling {
    /// `j` — penetrating-damage scale: how hard pen damage pushes severity up.
    pub pen_damage_scale:     PenDamageScale,
    /// `k` — Toughness mitigation: how much the defender's Toughness subtracts.
    pub toughness_mitigation: ToughnessMitigation,
    /// `I` — shooter-luck scale: the shooter's Luck adds (nastier wounds).
    pub shooter_luck_scale:   ShooterLuckScale,
    /// `L` — defender-luck scale: how far the defender's Luck extends the roll's
    /// floor below 0 (`roll(−L × Luck_defender .. R)`).
    pub defender_luck_scale:  DefenderLuckScale,
    /// `R` — the fixed ceiling (upper bound) of the random roll.
    pub random_spread:        RandomSpread,
    /// The ascending bucket edges `e0..e3` that tier the score into a severity.
    pub edges:                SeverityEdges,
}

impl Default for SeverityScaling {
    fn default() -> Self {
        // TUNABLE placeholder magnitudes for the resolution.md §6 symbols — the
        // forms are fixed, these numbers are balance data (de-brittled: tests
        // exercise the serde mechanism, not these shipped values).
        Self {
            pen_damage_scale:     PenDamageScale::new(1.0),
            toughness_mitigation: ToughnessMitigation::new(1.0),
            shooter_luck_scale:   ShooterLuckScale::new(1.0),
            defender_luck_scale:  DefenderLuckScale::new(1.0),
            random_spread:        RandomSpread::new(10.0),
            edges:                SeverityEdges::default(),
        }
    }
}
