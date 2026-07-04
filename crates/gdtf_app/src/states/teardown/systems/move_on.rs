use bevy::{prelude::*, window::PrimaryWindow};

/// Terminal exit for the [`Teardown`](crate::states::AppState::Teardown) scene:
/// quit the app once teardown work has completed.
///
/// WHY two exit paths (GTW-311 / Bevy issue #23313): on macOS in Bevy 0.18.1, an
/// `AppExit` message written from an ordinary system does NOT reliably terminate the
/// winit event loop — the timing is dependent on when the runner next polls, so the
/// app intermittently hangs on shutdown ("Entered Teardown State" then never exits).
/// The macOS-safe exit is to despawn the `PrimaryWindow` entity, which drives winit's
/// native window-close exit path (it does not have the #23313 bug — this is the
/// documented fallback at `app/capture/plugin.rs`). The `AppExit::Success` message is
/// KEPT for the headless / no-window / CI path, where there is no window to despawn and
/// the runner exits on the message instead. Both together: a windowed macOS run exits
/// via window-close; a headless run exits via `AppExit`.
///
/// `PrimaryWindow` access is panic-free: a [`Query`] yields zero entities headlessly
/// (the `despawn` loop simply does nothing) — a bare `Single` would panic on zero
/// matches.
///
/// SCAFFOLD DIVERGENCE (GTW-575): this `move_on` stays BESPOKE rather than collapsing
/// into `scaffold::advance_state_to` because it is not a `NextState::set` at all — it is
/// the app's TERMINAL exit, and it needs BOTH exit paths above (the windowed
/// window-despawn and the headless `AppExit` message), which the one-shape scaffold
/// deliberately does not grow flags for (P9).
pub(in crate::states::teardown) fn move_on(
    mut commands: Commands,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    // Windowed (macOS) exit: despawn the primary window to take winit's native
    // window-close path, sidestepping the #23313 `AppExit` hang. Zero matches headlessly.
    for window in &windows {
        commands.entity(window).despawn();
    }
    // Headless / no-window / CI exit: there is no window to despawn, so the runner
    // terminates on the `AppExit` message instead.
    exit.write(AppExit::Success);
}
