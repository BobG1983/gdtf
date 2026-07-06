//! The route-search **result types** (E7 · GTW-12d): the typed [`Path`] a
//! successful search returns, the typed [`PathBlocked`] no-route result, and the
//! accumulated-cost newtype [`PathCost`] the search relaxes with.
//!
//! These are pure data — no Bevy, no `World`, no RNG. They are what the route
//! core ([`super::core`]) and its two entry points ([`super::find_path`] /
//! [`super::reachable_within`]) hand back.

use crate::{ganger::Tu, metric::CellLevel};

/// An **accumulated** path cost — the running sum of per-step [`Tu`] edge costs as
/// the search relaxes the frontier.
///
/// A named newtype over `u32` (no-bare-types: an accumulated route cost is a domain
/// value, not a bare integer). It is **wider** than a single-step [`Tu`] (a `u8`) on
/// purpose: a multi-storey route over the 60×60×8 grid can sum many steps, so the
/// search accumulates in a `u32` that cannot wrap mid-search the way summing into a
/// `u8` would. The frontier orders by this cost FIRST (then the `(z, y, x)` cell key
/// — see `super::core`), and [`reachable_within`](super::reachable_within) compares
/// it against the budget. Private inner + derived [`std::ops::Deref`] (house style).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PathCost(u32);

impl PathCost {
    /// The zero cost — the seed cost of the search's start node.
    pub const ZERO: Self = Self(0);

    /// Build an accumulated cost from its raw `u32` magnitude.
    ///
    /// The public constructor (house style) so the search core and tests can build a
    /// `PathCost` without reaching the private field.
    #[must_use]
    pub const fn new(cost: u32) -> Self {
        Self(cost)
    }

    /// This cost plus one per-step edge [`Tu`] — the single relaxation step.
    ///
    /// Widens the `u8` step cost to `u32` and adds it, so the running total never
    /// wraps mid-search. The one place a step cost folds into the accumulated total,
    /// so the route total is exactly the sum of the edges relaxed along it (the §48
    /// bit-identity).
    #[must_use]
    pub fn add_step(self, step: Tu) -> Self {
        Self(self.0 + u32::from(*step))
    }

    /// Narrow the accumulated cost back to a [`Tu`] for the [`Path`] total.
    ///
    /// An in-domain route is bounded by a ganger's `u8` TU budget (≤ 255), so a
    /// route a ganger could ever afford fits a [`Tu`] exactly — and the §48
    /// bit-identity (the total equals the summed per-step [`Tu`]) holds bit-for-bit.
    /// A pathological route whose total exceeds `u8::MAX` SATURATES at `u8::MAX`
    /// rather than wrapping (no silent overflow); such a route is unaffordable and
    /// never committed.
    #[must_use]
    pub const fn to_tu(self) -> Tu {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "saturated to u8::MAX above before the cast, so it cannot truncate or wrap"
        )]
        let narrowed = if self.0 > u8::MAX as u32 {
            u8::MAX
        } else {
            self.0 as u8
        };
        Tu::new(narrowed)
    }
}

/// A successful route the search found — the ordered cells from `start` to `goal`
/// (inclusive on both ends), the per-step entry costs, and the route's total [`Tu`]
/// cost.
///
/// The typed result of [`find_path`](super::find_path) (C3). The
/// [`cells`](Path::cells) list runs `start..=goal` in step order (the first element
/// is `start`, the last is `goal`); the [`steps`](Path::steps) list carries the
/// per-step ENTRY cost ALIGNED to `cells[1..]` (`steps[i]` is the cost of entering
/// `cells[i + 1]` from `cells[i]`, so `steps.len() == cells.len() - 1`); the
/// [`total`](Path::total) is the sum of those per-step costs.
///
/// **§48 bit-identity (GTW-355).** The per-step costs are NOT re-derived by re-running
/// the cost math — they are the SAME accumulated-cost deltas the search relaxed with
/// (`cost(cells[i + 1]) − cost(cells[i])`, the exact
/// [`PathCost::add_step`](PathCost::add_step) increment along the reconstructed route),
/// so the stepped walk that charges [`steps`](Path::steps) one cell at a time sums to
/// [`total`](Path::total) **bit-for-bit by construction**. A zero-length route
/// (`start == goal`) is the one-cell path `[start]` with no steps at total `0`.
///
/// Private fields + read accessors (house style — a `Path` is built only by the
/// search core, never field-assembled from outside).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// The ordered route cells, `start..=goal` in step order (≥ 1 element).
    cells: Vec<CellLevel>,
    /// The per-step ENTRY costs, aligned to `cells[1..]`: `steps[i]` is the [`Tu`] cost
    /// of entering `cells[i + 1]` from `cells[i]`. Empty for the degenerate one-cell
    /// route. Their sum equals [`total`](Path::total) (the §48 bit-identity).
    steps: Vec<Tu>,
    /// The route's total cost — the sum of the per-step entry [`Tu`] along it.
    total: Tu,
}

impl Path {
    /// Build a route from its ordered `start..=goal` cells, per-step entry costs, and
    /// total cost — the constructor the search core calls once it has reconstructed the
    /// route.
    ///
    /// `steps` MUST be aligned to `cells[1..]` (one entry per traversed edge, so
    /// `steps.len() == cells.len().saturating_sub(1)`) and sum to `total` — the
    /// invariant the search core upholds by reconstructing each step from the settled
    /// accumulated-cost deltas (the §48 bit-identity).
    #[must_use]
    pub const fn new(cells: Vec<CellLevel>, steps: Vec<Tu>, total: Tu) -> Self {
        Self {
            cells,
            steps,
            total,
        }
    }

    /// The ordered route cells, `start..=goal` in step order — read-only.
    #[must_use]
    pub fn cells(&self) -> &[CellLevel] {
        &self.cells
    }

    /// The per-step ENTRY costs aligned to `cells[1..]` — read-only.
    ///
    /// `steps()[i]` is the [`Tu`] charged to enter `cells()[i + 1]` from `cells()[i]`;
    /// the slice is empty for a degenerate one-cell route. The GTW-355 stepped walk
    /// charges these one cell at a time as it advances, and their sum is
    /// [`total`](Path::total) bit-for-bit (the §48 identity), so an uninterrupted walk
    /// spends exactly the preview's price and an interrupted one spends exactly the
    /// ground it covered.
    #[must_use]
    pub fn steps(&self) -> &[Tu] {
        &self.steps
    }

    /// The route's total [`Tu`] cost — the sum of the per-step edge costs along it
    /// (the §48 bit-identity total).
    #[must_use]
    pub const fn total(&self) -> Tu {
        self.total
    }

    /// The route's start cell — the first cell of the ordered route (always present:
    /// a `Path` is never empty, the degenerate route is `[start]`).
    #[must_use]
    pub fn start(&self) -> Option<CellLevel> {
        self.cells.first().copied()
    }

    /// The route's goal cell — the last cell of the ordered route.
    #[must_use]
    pub fn goal(&self) -> Option<CellLevel> {
        self.cells.last().copied()
    }

    /// The number of cells in the route (`start..=goal` inclusive) — `1` for the
    /// degenerate `start == goal` route, `n + 1` for an `n`-step route.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the route has no cells. A search-produced `Path` is NEVER empty (the
    /// degenerate route is `[start]`), so this is `false` for any real route; it
    /// exists to satisfy the `len`/`is_empty` clippy pairing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// The typed **no-route** result — there is no legal route from `start` to `goal`
/// over the grids (C3).
///
/// A named unit error (no-bare-types: "blocked" is a domain outcome, not a bare
/// `()`/empty `Vec`) so [`find_path`](super::find_path) returns
/// `Result<`[`Path`]`, PathBlocked>` — a NO-route is the `Err` arm, NEVER a panic and
/// NEVER an empty [`Path`] (an empty path would be ambiguous with the degenerate
/// `start == goal` route). It carries no payload: the only fact is that the goal is
/// unreachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathBlocked;
