use super::support::{App, BleedingOut, MinimalPlugins};

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
