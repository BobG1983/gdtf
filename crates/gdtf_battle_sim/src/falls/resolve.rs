//! The pure **drop-resolution** verb (GTW-523 C2) — [`resolve_drop`], which scans a cell's
//! slab column downward from a faller's start storey to the storey it lands on.
//!
//! Render-free, `SurfaceGrid`-only, no ECS: given the `(cell, start)` a faller stands at and
//! the persistent [`SurfaceGrid`](crate::surface::SurfaceGrid), it returns the highest storey
//! `k < start` that SUPPORTS the faller — `k == 0` (ground always supports) OR
//! `slab_state((cell, k)) == Present`. A [`SlabState::Absent`](crate::surface::SlabState)
//! slab is open air (keep falling); a [`SlabState::Destroyed`](crate::surface::SlabState) one
//! is not support (a smashed floor is a hole).

use crate::{
    falls::StoreysFallen,
    metric::{Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

/// The resolved **landing** of one fall — the storey the faller lands on and the storey
/// distance it fell (GTW-523 C2).
///
/// A `Copy` value object of named domain types: the [`landing`](DropLanding::landing) storey
/// (the highest supported `k < start`) and the [`storeys`](DropLanding::storeys) it fell
/// (`start − landing`, always `≥ 1`). The falls system rewrites the faller's
/// [`Position`](crate::ganger::Position) to `(cell, landing)` and multiplies
/// `per_storey_damage` by `storeys`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DropLanding {
    /// The storey the faller lands ON — the highest `k < start` that supports it (ground
    /// `k == 0`, else the first `Present` slab scanning down).
    pub landing: Level,
    /// How many storeys it fell — `start − landing`, always `≥ 1` (a fall drops at least one
    /// storey: the destroyed slab it stood on is not support, so `start` itself is skipped).
    pub storeys: StoreysFallen,
}

/// Resolve the storey a faller lands on when the slab under it at `(cell, start)` is
/// destroyed — the pure §Falls drop scan (GTW-523 C2).
///
/// Scans `k` **downward** from `start − 1`: the first `k` that SUPPORTS the faller is the
/// landing storey. A storey `k` supports iff `k == 0` (the ground always supports) OR the
/// slab keyed `(cell, k)` reads [`SlabState::Present`] (an intact floor). A
/// [`SlabState::Absent`] slab is open air between storeys (keep falling); a
/// [`SlabState::Destroyed`] slab is a smashed floor — a hole, NOT support (keep falling). The
/// scan always terminates because `k == 0` supports unconditionally.
///
/// The distance is `start − landing`, always `≥ 1`: `start` itself is never a landing
/// candidate (the faller was standing on the floor of `start`, i.e. the slab keyed
/// `(cell, start)`, which was just destroyed — the caller only calls this when that slab is
/// gone), so the scan begins at `start − 1`. A faller already on the ground (`start == 0`)
/// cannot fall — the caller filters those out (there is no `k < 0` to land on), and this
/// verb is only invoked for `start ≥ 1`.
///
/// Returns [`None`] iff `start == 0` (a defensive guard — the ground has nowhere lower to
/// fall to); every `start ≥ 1` returns [`Some`] a landing (the ground `k == 0` is the
/// backstop). Pure: reads the [`SurfaceGrid`] and mutates nothing.
#[must_use]
pub fn resolve_drop(cell: Cell, start: Level, surface: &SurfaceGrid) -> Option<DropLanding> {
    let start_z = *start;
    if start_z == 0 {
        // On the ground already — nothing supports a fall below storey 0 (defensive; the
        // caller's faller predicate never keys a fall to level 0 destroyable-slab context,
        // but guard rather than underflow the scan).
        return None;
    }

    // Scan k downward from start-1. Land on the highest k that SUPPORTS: k == 0 (ground) OR
    // an intact Present slab. Absent = open air (fall on); Destroyed = a hole (fall on).
    let mut k = start_z - 1;
    loop {
        let supports = k == 0
            || surface.slab_state(&CellLevel::new(cell, Level::new(k))) == SlabState::Present;
        if supports {
            let landing = Level::new(k);
            // Distance is start − landing, always ≥ 1 (start_z ≥ 1 and k < start_z).
            let storeys = StoreysFallen::new(start_z - k);
            return Some(DropLanding { landing, storeys });
        }
        // Not support — keep falling. k == 0 already supported above, so k ≥ 1 here and the
        // decrement never underflows.
        k -= 1;
    }
}
