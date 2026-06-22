//! Tests for the GTW-340 squad fog model, split by acceptance criterion. Shared
//! fixtures live in [`support`]. No `App`, no RNG — hand-built grids (the sim-unit
//! idiom): construct observer component sets + grids + tuning, call
//! [`union_fov`](crate::visibility::union_fov) / [`accrue`](crate::visibility::accrue)
//! directly, and read the pure seams.

mod support;

mod accrual;
mod banded_occupant;
mod candidate_bound;
mod conscious_filter;
mod dense_floor;
mod reads;
mod union;
mod walled;
