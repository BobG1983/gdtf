//! AC2 — [`SimActsPlugin`] registers each `*Requested` buffer (a `MessageReader` passes
//! param-validation after update); the test reaches a post-update assert with no
//! validation panic.

use super::support::*;

#[test]
fn plugin_registers_every_message_buffer() {
    use bevy::prelude::{MessageReader, ResMut, Resource};

    /// A probe counter each reader-probe system bumps to prove it RAN (so its
    /// `MessageReader` param was validated by the scheduler, not skipped). One
    /// counter, not eight bools, so all eight probes increment the same resource.
    #[derive(Resource, Default)]
    struct Probed(u8);

    let mut app = headless_app();
    app.insert_resource(Probed::default());
    // One probe system per message type — each takes a `MessageReader<T>`, which the
    // scheduler param-validates against the registered buffer. An UNregistered buffer
    // would fail that validation; reaching the post-update assert (all eight probes ran)
    // proves all eight are registered by `SimActsPlugin`.
    app.add_systems(
        Update,
        (
            |mut r: MessageReader<FireRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
            |mut r: MessageReader<SetAimingRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
            |mut r: MessageReader<SetStanceRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
            |mut r: MessageReader<SetFacingRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
            |mut r: MessageReader<StabilizeDownedRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
            |mut r: MessageReader<ExecuteDownedRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
            |mut r: MessageReader<MoveRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
            |mut r: MessageReader<ReloadRequested>, mut p: ResMut<Probed>| {
                for _ in r.read() {}
                p.0 += 1;
            },
        ),
    );

    app.update();

    let ran = app.world().get_resource::<Probed>().map(|p| p.0);
    assert_eq!(
        ran,
        Some(8),
        "every `MessageReader<*Requested>` must pass param-validation — all eight \
         buffers are registered by `SimActsPlugin`",
    );
}
