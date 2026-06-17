//! AC5 — the `Stabilized` flag is DEFINED and OWNED here (re-exported from
//! `lib.rs`): it round-trips through a spawned entity carrying only the flag.

use super::support::{App, MinimalPlugins, Stabilized};

/// AC5 — the `Stabilized` flag is DEFINED and OWNED here (re-exported from
/// `lib.rs`): spawn an entity carrying `Stabilized` alone and query it back. This
/// is the home-of-the-flag proof — an entity with no other ganger component still
/// carries and yields its `Stabilized`. (The deeper re-export/Deref pins live in
/// `ganger::tests`; this confirms it round-trips through a spawned entity here.)
#[test]
fn stabilized_is_owned_here_and_queryable_alone() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let ganger = app.world_mut().spawn(Stabilized::new(true)).id();

    let flag = app.world().get::<Stabilized>(ganger).copied();
    assert_eq!(
        flag,
        Some(Stabilized::new(true)),
        "an entity carrying Stabilized alone must query its flag back (the flag's home)",
    );
}
