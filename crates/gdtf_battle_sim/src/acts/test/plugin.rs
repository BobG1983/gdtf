use super::support::*;

#[test]
fn plugin_registers_every_message_buffer() {
    use bevy::prelude::{MessageReader, ResMut, Resource};

    #[derive(Resource, Default)]
    struct Probed(u8);

    let mut app = headless_app();
    app.insert_resource(Probed::default());
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
