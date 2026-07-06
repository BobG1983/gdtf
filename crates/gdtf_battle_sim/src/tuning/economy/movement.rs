//! Terrain-determined step costs plus the vertical-link hop cost: [`MoveCost`],
//! the per-terrain [`MoveCosts`] table, and [`LinkTu`].

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::occupancy::TerrainKind;

/// One terrain's **move TU cost** — the flat number of Time Units a ganger spends to
/// step ONTO a cell of a given [`TerrainKind`] (`docs/combat/combat.md` L34
/// affirmatively lists "step" among the actions that "cost TUs").
///
/// The per-terrain cost the movement walk ([`crate::acts::movement::advance_walk`])
/// charges via [`crate::tu::spend_tu`] when a step lands on a cell — the move cost is
/// **terrain-determined** (the floor tile crossed determines the cost), so this is the
/// cost looked up from the destination cell's terrain via [`MoveCosts::cost`], NOT a flat
/// per-cell constant. The shared per-terrain newtype (the [`WoundCost`](crate::tuning::WoundCost)
/// / [`StanceContribution`](crate::tuning::StanceContribution) shared-newtype pattern):
/// every per-terrain field of [`MoveCosts`] is a `MoveCost`. A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type so the economy subtracts it directly (the verb
/// converts it to a [`Tu`](crate::ganger::Tu) via `Tu::new(*cost)` at the lookup site).
/// The default is a **starting point**, tunable
/// balance data — tests assert only the relation to this value (the drop equals the
/// looked-up terrain cost), never a pinned magnitude. `#[serde(transparent)]` lets it
/// parse a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct MoveCost(u8);

impl MoveCost {
    /// Build a per-terrain move TU cost from its flat Time-Unit magnitude (a starting
    /// point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style)
    /// while letting the `move_acts` tests and any programmatic tuning edit build a cost
    /// without a bare `u8` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// The **per-terrain move-cost table** — the move TU cost keyed by the destination
/// cell's [`TerrainKind`] (the data-driven, terrain-determined step cost,
/// `docs/combat/combat.md` L34: "step" costs TUs).
///
/// The single lookup choke point the movement walk
/// ([`crate::acts::movement::advance_walk`]) reads: it asks
/// [`OccupancyGrid::terrain`](crate::occupancy::OccupancyGrid::terrain) for the
/// destination's [`TerrainKind`] and looks the cost up here via [`MoveCosts::cost`]. One
/// [`MoveCost`] field per [`TerrainKind`] variant (the [`WoundCosts`](crate::tuning::WoundCosts)
/// / [`StanceStability`](crate::tuning::StanceStability) keyed-struct + per-field
/// precedent). The cost data is a SIM concept (the TU economy) and lives in the sim's
/// tuning, NOT the presenter's view-only tile data.
///
/// NOTE on which terrains can be a destination: a standing [`TerrainKind::Wall`] /
/// [`TerrainKind::Cover`] cell is blocked, so the verb's `is_blocked` gate rejects it
/// before this lookup — `wall` is effectively dead under the gate (kept for total
/// coverage / future use), and `cover` applies only to a destroyed-cover cell (whose
/// terrain still reads [`TerrainKind::Cover`] but no longer blocks). The granularity is
/// per-[`TerrainKind`] (coarse — Open / Cover / Wall), NOT per-floor-TYPE; richer
/// per-floor-type movement costs need a richer sim terrain model (a follow-up). The
/// magnitudes are a **starting point**, tunable balance data — tests assert only the
/// relation (the drop equals the looked-up terrain's cost), never a pinned magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MoveCosts {
    /// The move TU cost to step onto an [`TerrainKind::Open`] cell — the cheapest
    /// baseline (doc starting point: the floor of the table).
    pub open:  MoveCost,
    /// The move TU cost to step onto a [`TerrainKind::Cover`] cell — dearer than open;
    /// reachable only when the cover has been destroyed (a standing cover blocks).
    pub cover: MoveCost,
    /// The move TU cost to step onto a [`TerrainKind::Wall`] cell — a placeholder; a wall
    /// is never a legal destination (it blocks), so this entry is dead under the verb's
    /// gate (kept for total terrain coverage / future use).
    pub wall:  MoveCost,
}

impl MoveCosts {
    /// The move TU cost for a destination cell of the given [`TerrainKind`] — the single
    /// lookup the movement verb calls, mapping each variant to its field.
    ///
    /// The verb reads the destination cell's terrain
    /// ([`OccupancyGrid::terrain`](crate::occupancy::OccupancyGrid::terrain)) and asks
    /// this for the per-terrain cost; it converts the returned [`MoveCost`] to a
    /// [`Tu`](crate::ganger::Tu) (`Tu::new(*cost)`) for the `can_spend_tu` / `spend_tu`
    /// calls. Exhaustive over the
    /// three [`TerrainKind`] variants — no fallback, so a new variant is a compile error
    /// here (forcing the table to grow with the terrain model).
    ///
    /// [`Emplacement`](TerrainKind::Emplacement) reuses the `wall` cost (GTW-543): an
    /// emplacement is impassable blocking geometry a move can never step ONTO (a ganger
    /// ENTERS it via the dedicated `EnterEmplacementTu`-costed act, never by walking into
    /// its cell), so this lookup is never charged for an emplacement cell; it maps to the
    /// impassable `wall` cost rather than growing the RON table with an unreachable field.
    #[must_use]
    pub const fn cost(&self, terrain: TerrainKind) -> MoveCost {
        match terrain {
            TerrainKind::Open => self.open,
            TerrainKind::Cover => self.cover,
            TerrainKind::Wall | TerrainKind::Emplacement => self.wall,
        }
    }
}

impl Default for MoveCosts {
    fn default() -> Self {
        // Value-agnostic STARTING POINTS (tunable balance data), mirroring the
        // TurnTu / StanceChangeTu pattern — the user gave a per-step facing cost but NOT
        // per-terrain move costs, so these are flagged tunables, never balance numbers.
        // Open is the cheapest baseline; Cover (reachable only when destroyed) is dearer;
        // Wall is a placeholder (a wall is never a legal destination). Tests assert only
        // the relation (the drop equals the looked-up terrain's cost), never a magnitude.
        Self {
            open:  MoveCost(4),
            cover: MoveCost(6),
            wall:  MoveCost(8),
        }
    }
}

/// The **per-link traversal TU cost** — the flat number of Time Units a ganger spends to
/// cross a single vertical link (a stair / ladder hop between levels;
/// `docs/combat/visibility.md` §48: "`advance_walk` ... charges ... the flat `link_tu`
/// at a link hop (a crossing prices `link_tu` *instead of* terrain)").
///
/// One FLAT cost for every link kind — the canon prices a crossing at a single `link_tu`,
/// NOT a per-kind table (no `stair_tu` / `ladder_tu` split; OQ-6's per-kind variant would
/// require a docs change first). When a step is a link hop the movement verb charges THIS
/// instead of the destination cell's [`MoveCost`] — the crossing's cost is the link's, not
/// the terrain's. A small `u8` count, matching [`crate::ganger::Tu`]'s inner type so the
/// economy subtracts it directly. The default is a **starting point**, tunable balance data
/// — tests assert only the relation to this value (the drop equals it), never the magnitude.
/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner + derived [`Deref`].
///
/// **Consumed later by GTW-351** (the vertical-link traversal verb) — this leaf only ADDS
/// the tunable; no movement code reads it yet.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LinkTu(u8);

impl LinkTu {
    /// Build a per-link traversal TU cost from its flat Time-Unit magnitude (a starting
    /// point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style)
    /// while letting tests and any programmatic tuning edit build a cost without a bare
    /// `u8` escaping; shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for LinkTu {
    fn default() -> Self {
        // A flat 4 TU to cross one vertical link — a STARTING POINT (tunable balance data),
        // matched to the cheapest move-cost baseline (a link hop is one "step" of effort).
        // ONE flat cost for every link kind (no per-kind split). Value-agnostic tests only,
        // never a pinned magnitude.
        Self(4)
    }
}
