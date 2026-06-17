//! The DEV-ONLY auto-enter-battle affordance (GTW-223). The gate, witness, plugin, and
//! drive systems live in the `plugin` submodule; see it for the full rationale. Tests
//! live in the sibling `test` submodule (GTW-201).

mod plugin;

// `AutoBattlePlugin` is the only item the binary consumes (via `gdtf_app.rs`), so it
// is re-exported in BOTH configurations, at the same `test-support` visibility flip
// the item itself uses (`support_item!` in `plugin`): `pub` under `test-support` (the
// `test_support` re-export needs it), `pub(crate)` otherwise — `unreachable_pub`-clean
// either way.
crate::support_use!(plugin::AutoBattlePlugin;);

// `AutoBattleActive` + `auto_battle_enabled` are consumed ONLY through the
// `test_support` re-export (the AC1 tests), which is itself `#[cfg(test-support)]`. The
// binary never names them, so re-exporting them in a non-`test-support` build would be
// an unused `pub(crate) use` (caught by `cargo dbuild`, which builds the binary WITHOUT
// `test-support`). Gate the re-export to the same feature, `pub` because the
// `test_support` re-export needs it.
#[cfg(feature = "test-support")]
pub use plugin::{AutoBattleActive, auto_battle_enabled};

#[cfg(test)]
mod test;
