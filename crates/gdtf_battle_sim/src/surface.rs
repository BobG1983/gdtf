//! Persistent surface grid: the model's authoritative store of floor/roof **slab**
//! existence and per-cell **ground** damage — battle *state* carried across every
//! occupancy rebuild. The **persistent surface grid**: floor/roof slabs + ground
//! records are battle *state*, carried across every rebuild, so a destroyed slab
//! stays destroyed and ground damage accrues (a model-authoritative store in the
//! model/view split — ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! This is the E1.5 surface-grid slice. It is a **separate** resource from the
//! coarse occupancy (E1.6 / GTW-156, not built yet): the surface grid carries slab
//! *existence* and ground *damage* ONLY — occupant presence is the occupancy grid's
//! job and is out of scope here. The occupancy grid is **rebuilt fresh per shot**;
//! the surface grid is **mutated in place** and survives that rebuild, which is the
//! whole reason it is a distinct, persistent store.
//!
//! Two persistence invariants, each modelled so the type *cannot* violate it:
//!
//! 1. **A destroyed slab stays destroyed.** [`SlabState`] is `Present` / `Destroyed`
//!    / `Absent`; [`SurfaceGrid::destroy_slab`] sets `Destroyed` and there is **no
//!    API that reverts it** — [`SurfaceGrid::set_slab`] refuses to overwrite a
//!    `Destroyed` entry (a destroyed slab stays destroyed; slab destroyed at zero).
//!    Destruction is permanent by construction.
//! 2. **Ground is damaged, never destroyed, and damage only accrues.**
//!    [`GroundDamage`] is a `u32` accumulator; [`SurfaceGrid::accrue_ground_damage`]
//!    is **additive (saturating)** so the total is monotonically non-decreasing —
//!    there is no API that lowers it, and the guarded [`SurfaceGrid::set_ground_damage`]
//!    rejects any value below the current total (the ground-hit verb: damaged,
//!    never destroyed).
//!
//! The slab key is the E1.1 [`CellLevel`] (the `(cell, level)` of the slab between
//! storeys); the ground key is the E1.1 [`Cell`] (the ground plane has no storey).
//! Both maps are lazily populated: an absent slab key reads as [`SlabState::Absent`]
//! (no slab authored there) and an absent ground key reads as zero damage.

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
};

use crate::metric::{Cell, CellLevel};

/// The state of one floor/roof slab at a `(cell, level)` — does a slab exist there,
/// and if it did, has it been destroyed?
///
/// A named domain enum (no-bare-types: slab existence is a domain value, not a bare
/// `Option<bool>`). The three states are mutually exclusive and `Destroyed` is
/// **terminal** — see [`SurfaceGrid::destroy_slab`]:
///
/// - [`Present`](SlabState::Present): an intact slab — it stops a round crossing the
///   z-boundary (`docs/combat/resolution.md`: "an intact slab stops the round").
/// - [`Destroyed`](SlabState::Destroyed): a slab that existed and was smashed at zero
///   HP — **permanent**, it can never revert to `Present` or `Absent` (a destroyed
///   slab stays destroyed).
/// - [`Absent`](SlabState::Absent): no slab was ever authored here (open air between
///   storeys) — the default read for an untouched key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlabState {
    /// An intact slab — it stops a round crossing this z-boundary.
    Present,
    /// A slab smashed to zero — **permanent**; it never reverts to `Present`/`Absent`.
    Destroyed,
    /// No slab authored here (open air between storeys) — the default for an
    /// untouched key.
    Absent,
}

impl SlabState {
    /// Whether this slab has been destroyed — the terminal state that no API can
    /// revert.
    #[must_use]
    pub const fn is_destroyed(self) -> bool {
        matches!(self, Self::Destroyed)
    }
}

/// Accumulated damage dealt to the ground at one [`Cell`] — a monotonically
/// non-decreasing `u32` accumulator.
///
/// A named newtype over `u32` (no-bare-types: a ground-damage total is a domain
/// value distinct from any HP pool or cover-damage amount — rule 3). The ground is
/// **damaged, never destroyed** (the ground-hit verb: damaged, never destroyed),
/// so this only ever grows: [`SurfaceGrid`] exposes no
/// API that lowers it. Private inner + derived [`Deref`] (house style). A magnitude
/// is per-hit gameplay data, not pinned here.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct GroundDamage(u32);

impl GroundDamage {
    /// Build a ground-damage total from its magnitude (per-hit gameplay data).
    #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }

    /// Accumulate `amount` more damage, **saturating** at `u32::MAX` — the additive
    /// step that makes ground damage monotonically non-decreasing by construction
    /// (it can only ever grow, never shrink).
    #[must_use]
    pub const fn accrue(self, amount: Self) -> Self {
        Self(self.0.saturating_add(amount.0))
    }
}

/// The persistent surface grid — the sim's **single authoritative store** of
/// floor/roof slab existence and per-cell ground damage, carried across every
/// occupancy rebuild (the persistent surface grid — a model-authoritative store of
/// the "Surfaces / ground" facts in the model/view split; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A Bevy [`Resource`] (one grid per battle), **mutated in place** — never rebuilt.
/// It is deliberately a **separate** resource from the coarse occupancy (E1.6 /
/// GTW-156): occupancy is rebuilt fresh per shot, while this carries the durable
/// facts (a destroyed slab, accrued ground damage) that must survive that rebuild.
///
/// Both inner maps are **lazily populated**:
/// - an absent slab key reads as [`SlabState::Absent`] (no slab authored there);
/// - an absent ground key reads as [`GroundDamage`] zero (no ground hit there yet).
#[derive(Resource, Debug, Clone, Default)]
pub struct SurfaceGrid {
    /// Per-`(cell, level)` floor/roof slab state. Absent keys read as
    /// [`SlabState::Absent`]. A [`SlabState::Destroyed`] entry is permanent — see
    /// [`destroy_slab`](SurfaceGrid::destroy_slab) / [`set_slab`](SurfaceGrid::set_slab).
    slabs:  HashMap<CellLevel, SlabState>,
    /// Per-[`Cell`] accumulated ground damage. Absent keys read as zero; the total
    /// only ever grows — see [`accrue_ground_damage`](SurfaceGrid::accrue_ground_damage).
    ground: HashMap<Cell, GroundDamage>,
}

impl SurfaceGrid {
    /// Build an empty surface grid (no slabs authored, no ground hit yet — every
    /// slab key reads `Absent`, every ground key reads zero by lazy default).
    #[must_use]
    pub fn new() -> Self {
        Self {
            slabs:  HashMap::default(),
            ground: HashMap::default(),
        }
    }

    /// The slab state at `key`, defaulting to [`SlabState::Absent`] for an untouched
    /// key — a read-only peek that never mutates the grid.
    ///
    /// An absent key means "no slab authored here" ([`SlabState::Absent`]); this is
    /// the normal read path for both the march (is there an intact slab to stop the
    /// round?) and the persistence tests (is a destroyed slab still destroyed?).
    #[must_use]
    pub fn slab_state(&self, key: &CellLevel) -> SlabState {
        self.slabs.get(key).copied().unwrap_or(SlabState::Absent)
    }

    /// Set the slab at `key` to `state` — used at battle setup to author the
    /// situation's upper-floor slabs (`Present`) and gaps (`Absent`).
    ///
    /// **Permanence guard:** if the slab at `key` is already
    /// [`SlabState::Destroyed`], this is a **no-op** — a destroyed slab can never be
    /// reverted to `Present` or `Absent` (a destroyed slab stays destroyed). The only
    /// way a slab becomes `Destroyed` is
    /// [`destroy_slab`](SurfaceGrid::destroy_slab); this method exists for authoring
    /// the *pre-destruction* state and must not be a back door around permanence.
    pub fn set_slab(&mut self, key: CellLevel, state: SlabState) {
        if self.slab_state(&key).is_destroyed() {
            // Permanent destruction: refuse to overwrite a Destroyed slab.
            return;
        }
        self.slabs.insert(key, state);
    }

    /// Destroy the slab at `key` — set it [`SlabState::Destroyed`], **idempotently
    /// and permanently** (slab destroyed at zero; a destroyed slab stays destroyed).
    ///
    /// This is the surface-hit verb's outcome (a slab spent to zero HP). It is the
    /// ONLY way an entry becomes `Destroyed`, and the state is terminal: once set,
    /// no API — including [`set_slab`](SurfaceGrid::set_slab) and a repeat
    /// `destroy_slab` — can move it back to `Present`/`Absent`. Calling it on an
    /// already-destroyed slab is a harmless no-op (idempotent), and calling it on an
    /// `Absent` key still destroys it (a slab can be smashed before it was ever
    /// authored `Present` — the destroyed record is what matters).
    pub fn destroy_slab(&mut self, key: CellLevel) {
        self.slabs.insert(key, SlabState::Destroyed);
    }

    /// The accumulated ground damage at `cell`, defaulting to zero for a cell that
    /// has taken no ground hit — a read-only peek that never mutates the grid.
    #[must_use]
    pub fn ground_damage(&self, cell: &Cell) -> GroundDamage {
        self.ground.get(cell).copied().unwrap_or_default()
    }

    /// Accrue `amount` more ground damage at `cell`, returning the new total — the
    /// ground-hit verb (damaged, never destroyed).
    ///
    /// **Monotonic by construction:** the new total is the old total plus `amount`
    /// (saturating at `u32::MAX`), so it can only ever grow. There is no API to lower
    /// it; the ground is damaged, never destroyed and never repaired. Lazily seeds a
    /// previously-untouched cell from zero.
    pub fn accrue_ground_damage(&mut self, cell: Cell, amount: GroundDamage) -> GroundDamage {
        let total = self.ground_damage(&cell).accrue(amount);
        self.ground.insert(cell, total);
        total
    }

    /// Guarded direct set of the ground-damage total at `cell`, **rejecting any
    /// value below the current total** — returns `true` if it was applied, `false`
    /// if rejected.
    ///
    /// Ground damage is monotonically non-decreasing: a `value` at-or-above the
    /// current total is applied (it never lowers the accumulator), while a `value`
    /// below the current total is **rejected as a no-op**. The additive
    /// [`accrue_ground_damage`](SurfaceGrid::accrue_ground_damage) is the normal
    /// write path; this guarded setter exists for callers that hold an absolute
    /// total (and proves a decrease attempt cannot succeed — C4).
    pub fn set_ground_damage(&mut self, cell: Cell, value: GroundDamage) -> bool {
        if value < self.ground_damage(&cell) {
            // Monotonic guard: refuse to lower the accumulated total.
            return false;
        }
        self.ground.insert(cell, value);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metric::{Cell, Level};

    fn slab_key(x: i32, y: i32, level: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(level))
    }

    /// A no-op stand-in for an E1.6 occupancy update tick. The point of the C7(a)
    /// test is that running a tick does NOT rebuild or touch the persistent surface
    /// grid — the occupancy grid (GTW-156) is rebuilt per shot, the surface grid is
    /// not. With the occupancy grid not built yet, this no-op is a faithful
    /// stand-in: it takes the grid by shared ref and changes nothing.
    fn occupancy_update_tick(_grid: &SurfaceGrid) {
        // Intentionally empty: an occupancy tick must not mutate the surface grid.
    }

    /// C7(a) — slab destruction is PERMANENT across an occupancy tick, and re-setting
    /// it `Present`/`Absent` afterwards does NOT revert it.
    ///
    /// Destroys a slab, runs a no-op occupancy update tick, and asserts the slab is
    /// still `Destroyed`; then attempts `set_slab(Present)` and `set_slab(Absent)`
    /// and asserts both are rejected (the slab stays `Destroyed`). This pins the C2
    /// / C4 permanence invariant — a structural rule the acceptance criteria require.
    #[test]
    fn destroyed_slab_stays_destroyed_across_tick_and_resets() {
        let mut grid = SurfaceGrid::new();
        let k = slab_key(3, 4, 1);

        // Authored present, then destroyed.
        grid.set_slab(k, SlabState::Present);
        assert_eq!(grid.slab_state(&k), SlabState::Present);
        grid.destroy_slab(k);
        assert_eq!(grid.slab_state(&k), SlabState::Destroyed);

        // An occupancy update tick (no-op stand-in for GTW-156) must NOT rebuild or
        // touch the persistent surface grid.
        occupancy_update_tick(&grid);
        assert_eq!(
            grid.slab_state(&k),
            SlabState::Destroyed,
            "a destroyed slab must survive an occupancy tick (the grid is not rebuilt)",
        );

        // Attempting to revert via set_slab is a no-op — destruction is permanent.
        grid.set_slab(k, SlabState::Present);
        assert_eq!(
            grid.slab_state(&k),
            SlabState::Destroyed,
            "set_slab(Present) must NOT revert a destroyed slab",
        );
        grid.set_slab(k, SlabState::Absent);
        assert_eq!(
            grid.slab_state(&k),
            SlabState::Destroyed,
            "set_slab(Absent) must NOT revert a destroyed slab",
        );
    }

    /// `destroy_slab` is idempotent and works even on a never-authored (`Absent`)
    /// key — a slab record can be smashed regardless of its prior state, and a repeat
    /// destroy is a harmless no-op (still `Destroyed`).
    #[test]
    fn destroy_slab_is_idempotent_and_works_on_absent() {
        let mut grid = SurfaceGrid::new();
        let k = slab_key(0, 0, 2);

        // Never authored Present — reads Absent by lazy default.
        assert_eq!(grid.slab_state(&k), SlabState::Absent);

        grid.destroy_slab(k);
        assert_eq!(grid.slab_state(&k), SlabState::Destroyed);
        // Repeat destroy: still Destroyed (idempotent).
        grid.destroy_slab(k);
        assert_eq!(grid.slab_state(&k), SlabState::Destroyed);
    }

    /// An absent slab key reads `Absent`, and an authored `Present`/`Absent` slab
    /// (not yet destroyed) CAN still be re-set — the permanence guard only locks
    /// `Destroyed`, not the pre-destruction authoring states.
    #[test]
    fn set_slab_freely_mutates_until_destroyed() {
        let mut grid = SurfaceGrid::new();
        let k = slab_key(7, 7, 0);

        assert_eq!(grid.slab_state(&k), SlabState::Absent);
        grid.set_slab(k, SlabState::Present);
        assert_eq!(grid.slab_state(&k), SlabState::Present);
        // Present → Absent is allowed (authoring), since it is not yet Destroyed.
        grid.set_slab(k, SlabState::Absent);
        assert_eq!(grid.slab_state(&k), SlabState::Absent);
    }

    /// C7(b) — ground damage is MONOTONIC: accruing twice yields the SUM.
    ///
    /// Applies two arbitrary damage amounts to the same cell and asserts the stored
    /// total equals their sum. Arbitrary magnitudes — the invariant is "the total is
    /// the sum of accruals", not a specific number (not brittle).
    #[test]
    fn ground_damage_accrues_to_the_sum() {
        let mut grid = SurfaceGrid::new();
        let cell = Cell::new(5, 6);

        // Untouched cell reads zero.
        assert_eq!(grid.ground_damage(&cell), GroundDamage::new(0));

        let first = grid.accrue_ground_damage(cell, GroundDamage::new(13));
        assert_eq!(first, GroundDamage::new(13), "first accrual is the amount");
        let second = grid.accrue_ground_damage(cell, GroundDamage::new(9));
        assert_eq!(
            second,
            GroundDamage::new(22),
            "two accruals must total their sum (13 + 9)",
        );
        // And the stored total matches the returned running total.
        assert_eq!(grid.ground_damage(&cell), GroundDamage::new(22));
    }

    /// C7(b) the other side — a DECREASE is rejected (no-op): the guarded
    /// `set_ground_damage` refuses any value below the current total, and there is no
    /// additive path that can lower it.
    ///
    /// Accrues damage, then attempts to set a lower total and asserts it is rejected
    /// (returns `false`) and the stored total is unchanged; a value at-or-above the
    /// current total is accepted. Pins the C4 monotonic invariant.
    #[test]
    fn ground_damage_decrease_is_rejected() {
        let mut grid = SurfaceGrid::new();
        let cell = Cell::new(2, 2);

        grid.accrue_ground_damage(cell, GroundDamage::new(40));
        assert_eq!(grid.ground_damage(&cell), GroundDamage::new(40));

        // A decrease attempt is rejected and leaves the total untouched.
        let lowered = grid.set_ground_damage(cell, GroundDamage::new(10));
        assert!(
            !lowered,
            "setting a value below the current total must be rejected"
        );
        assert_eq!(
            grid.ground_damage(&cell),
            GroundDamage::new(40),
            "a rejected decrease must NOT change the stored total",
        );

        // A non-decreasing set is accepted (it never lowers the accumulator).
        let raised = grid.set_ground_damage(cell, GroundDamage::new(55));
        assert!(
            raised,
            "setting a value at-or-above the current total is accepted"
        );
        assert_eq!(grid.ground_damage(&cell), GroundDamage::new(55));
    }

    /// `GroundDamage::accrue` saturates at `u32::MAX` rather than overflowing —
    /// monotonicity is preserved at the ceiling (the total never wraps to a smaller
    /// value).
    #[test]
    fn ground_damage_accrue_saturates() {
        let near_max = GroundDamage::new(u32::MAX - 1);
        let total = near_max.accrue(GroundDamage::new(10));
        assert_eq!(
            total,
            GroundDamage::new(u32::MAX),
            "accrual must saturate at u32::MAX, never wrap below the current total",
        );
    }

    /// The ground/slab maps are independent: a slab key and a ground key do not
    /// collide, and the surface grid carries BOTH facts side by side.
    #[test]
    fn slab_and_ground_are_independent() {
        let mut grid = SurfaceGrid::new();
        let k = slab_key(1, 1, 3);
        let cell = Cell::new(1, 1);

        grid.destroy_slab(k);
        grid.accrue_ground_damage(cell, GroundDamage::new(7));

        assert_eq!(grid.slab_state(&k), SlabState::Destroyed);
        assert_eq!(grid.ground_damage(&cell), GroundDamage::new(7));
    }

    /// The `GroundDamage` derived [`Deref`] reaches its inner `u32`, and
    /// `SlabState::is_destroyed` reports the terminal state. Arbitrary literals — the
    /// Deref/predicate mechanism, not a magnitude.
    #[test]
    fn surface_newtypes_expose_inner() {
        assert_eq!(*GroundDamage::new(99), 99u32);
        assert!(SlabState::Destroyed.is_destroyed());
        assert!(!SlabState::Present.is_destroyed());
        assert!(!SlabState::Absent.is_destroyed());
    }
}
