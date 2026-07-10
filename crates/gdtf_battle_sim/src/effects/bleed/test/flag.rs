//! The [`BleedingOut`] condition is DEFINED and OWNED in this module (GTW-695,
//! re-exported from `lib.rs`): it round-trips through a spawned entity as a marker.

use super::support::{App, BleedingOut, MinimalPlugins};

/// The [`BleedingOut`] condition is a marker owned here: spawn an entity carrying it
/// alone and query it back. The home-of-the-condition proof — presence is the whole
/// signal, so an entity with no other component still carries and yields it.
#[test]
fn bleeding_out_is_owned_here_and_queryable_alone() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let ganger = app.world_mut().spawn(BleedingOut).id();

    assert!(
        app.world().get::<BleedingOut>(ganger).is_some(),
        "an entity carrying BleedingOut alone must query the condition back (its home)",
    );
}
