//! Crate-internal visibility-flip macros shared across `gdtf_app`.
//!
//! [`support_item`] / [`support_use`] declare an item / re-export a path with `pub`
//! visibility when the `test-support` feature is enabled and `pub(crate)` otherwise,
//! so the state enums, [`ScenesPlugin`](crate::states::ScenesPlugin), and the GTW-223
//! auto-battle affordance can widen to `pub` for the external test harness while
//! staying `pub(crate)` (and `unreachable_pub`-clean) in the production binary.
//!
//! # Where `support_use!` still climbs — and where it must NOT (GTW-569)
//!
//! Since GTW-569, panel/scene TEST-ONLY markers do **not** ride `support_use!` climbs
//! through the intermediate `mod.rs` files. Each panel owns ONE
//! `#[cfg(feature = "test-support")] pub(crate) mod test_support` submodule in its
//! `mod.rs`, and the crate-root ledger (`src/test_support.rs`) re-exports those items
//! directly by explicit name — exporting a new panel marker is exactly **2 edits**
//! (the panel's `test_support` submodule + one ledger entry; see the ledger's module
//! docs for the walk-through). The surviving `support_use!` call sites are exactly:
//!
//! - the unconditional state-enum climbs (the GTW-321 co-location contract keeps
//!   `crate::states::<Enum>` nameable at the states root), plus
//!   [`ScenesPlugin`](crate::states::ScenesPlugin) /
//!   [`LoadedSituation`](crate::states::LoadedSituation) at the states root;
//! - the dual-use re-exports the production binary also reads: the bottom-bar root
//!   (`set_world_viewport` measures it) and the auto-battle plugin
//!   (`crate::app::auto_battle`).

/// Declares an item with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise.
///
/// The state enums and [`states::ScenesPlugin`](crate::states::ScenesPlugin) are
/// crate-internal in normal builds (the production public API is exactly
/// [`GdtfApp`](crate::GdtfApp)). With `test-support` on, an external crate must be able
/// to *name* them through [`test_support`](crate::test_support), so their definitions
/// widen to `pub`. Wrapping each definition in this macro keeps the visibility flip in
/// one place and lets `unreachable_pub` stay satisfied in both configurations. It wraps
/// `enum` / `struct` item definitions, free / inherent `fn` items (the GTW-223
/// auto-battle affordance widens its `auto_battle_enabled` gate + its `from_env`
/// constructor this way), and associated `const` items (the GTW-428 `BaseAttribute::ALL`
/// / `DerivedStat::ALL` display-order arrays an external test iterates). The `const fn`
/// arm exists because clippy `missing_const_for_fn` (denied) forces a const-eligible
/// constructor to be `const` — a plain `fn` arm cannot express that (e.g.
/// `LoadedSituation::new`); the bare `const` arm follows it so `const fn` is matched
/// first.
#[cfg(feature = "test-support")]
macro_rules! support_item {
    ($(#[$meta:meta])* enum $($rest:tt)*) => { $(#[$meta])* pub enum $($rest)* };
    ($(#[$meta:meta])* struct $($rest:tt)*) => { $(#[$meta])* pub struct $($rest)* };
    ($(#[$meta:meta])* const fn $($rest:tt)*) => { $(#[$meta])* pub const fn $($rest)* };
    ($(#[$meta:meta])* const $($rest:tt)*) => { $(#[$meta])* pub const $($rest)* };
    ($(#[$meta:meta])* fn $($rest:tt)*) => { $(#[$meta])* pub fn $($rest)* };
}

/// Declares an item with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise. See the `test-support`
/// variant for the rationale.
#[cfg(not(feature = "test-support"))]
macro_rules! support_item {
    ($(#[$meta:meta])* enum $($rest:tt)*) => { $(#[$meta])* pub(crate) enum $($rest)* };
    ($(#[$meta:meta])* struct $($rest:tt)*) => { $(#[$meta])* pub(crate) struct $($rest)* };
    ($(#[$meta:meta])* const fn $($rest:tt)*) => { $(#[$meta])* pub(crate) const fn $($rest)* };
    ($(#[$meta:meta])* const $($rest:tt)*) => { $(#[$meta])* pub(crate) const $($rest)* };
    ($(#[$meta:meta])* fn $($rest:tt)*) => { $(#[$meta])* pub(crate) fn $($rest)* };
}

/// Re-exports a path with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise.
///
/// The `mod.rs` re-export of a widened item caps the item's visibility, so the
/// re-export chain that [`test_support`](crate::test_support) reaches through must
/// widen in lockstep with [`support_item`]; otherwise the public re-export hits E0365.
#[cfg(feature = "test-support")]
macro_rules! support_use {
    ($($rest:tt)*) => { pub use $($rest)* };
}

/// Re-exports a path with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise. See the `test-support`
/// variant for the rationale.
#[cfg(not(feature = "test-support"))]
macro_rules! support_use {
    ($($rest:tt)*) => { pub(crate) use $($rest)* };
}

pub(crate) use support_item;
pub(crate) use support_use;
