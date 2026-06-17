//! The [`SurfaceGrid`] resource and its value types — slab [`SlabState`] (with the
//! terminal `Destroyed` state) and the monotonic [`GroundDamage`] accumulator.

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
