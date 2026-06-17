use bevy::{camera::ClearColorConfig, prelude::*};

/// Marker for the persistent UI [`Camera2d`] owned by the app shell.
///
/// This is plumbing around the framework camera (the `Camera2d` itself is
/// exempt from the no-bare-types rule), not a domain value — it exists purely so
/// the survival test can name *this* camera unambiguously and so future systems
/// can target the UI camera without re-querying every `Camera2d` in the world.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct UiCamera;

/// Spawns the single persistent UI [`Camera2d`] on entry to
/// [`AppState::Running`](crate::states::AppState::Running).
///
/// # Lifecycle choice
///
/// The camera is spawned `OnEnter(AppState::Running)` and is owned here in the
/// app shell (`gdtf_app`), **not** in `gdtf_ui`. The UI camera's lifetime is an
/// app-shell concern tied to `AppState`, not a widget concern: every screen
/// under `RunningState` (`Menu`, `Game`, `Options`, `Quit`) renders against the
/// same camera, so its lifetime must span the whole `Running` phase rather than
/// any one sub-state.
///
/// # Persistence
///
/// The entity is spawned with **no** `DespawnOnExit` (or any other scene-scoped
/// cleanup marker), so it deliberately outlives every `RunningState` sub-state
/// transition — a `RunningState::Menu → Options → Game → Quit` walk leaves this
/// camera untouched. It lives until the world that holds it is torn down, which
/// is the intended behaviour for a UI camera shared across all running screens.
///
/// This camera is also distinct from the GTW-134 UI test harness camera
/// (`GdtfUiTestAppBuilder::with_ui_camera`): that one is spawned by the test
/// builder itself, never via this system, so the two are not coupled.
///
/// # Clear (GTW-271)
///
/// The UI camera's [`Camera::clear_color`] is [`ClearColorConfig::None`]: it renders at the
/// default `Camera.order` `0`, ABOVE the GTW-216 world camera (order `-1`), which already
/// cleared the whole surface to the margin bg and blitted the map into its viewport sub-rect.
/// `None` makes the UI camera composite its panels OVER that result WITHOUT clearing, so the
/// map stays visible in the central rect and the panels render in the margins — the GTW-271
/// confined-map recipe. (The default `ClearColorConfig::Default` would clear the surface to
/// the `ClearColor` resource, wiping the map the world camera just drew.)
pub(in crate::scenes::running) fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        UiCamera,
        Camera {
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}
