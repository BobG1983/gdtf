//! Headless **UI-layout + asset** test harness for the GDTF Bevy app.
//!
//! [`GdtfUiTestAppBuilder`] is the sibling of
//! [`GdtfTestAppBuilder`](crate::GdtfTestAppBuilder) for tests that need real
//! `bevy_ui` **layout geometry** (a computed [`ComputedNode`]) or a live
//! [`AssetServer`] — neither of which exists under the `MinimalPlugins` app that
//! [`GdtfTestAppBuilder`](crate::GdtfTestAppBuilder) builds.
//!
//! It runs `DefaultPlugins` in the official `no_renderer.rs` headless
//! configuration:
//!
//! - [`RenderPlugin`] with [`WgpuSettings`]`{ backends: None, .. }` — the render
//!   app initialises with **no GPU adapter**, so it runs on a CI box with no GPU.
//! - [`WinitPlugin`] **disabled** — no event loop / OS window is created.
//! - [`WindowPlugin`]`{ primary_window: None, exit_condition: DontExit, .. }` — no
//!   window entity is spawned and the app does not try to exit when there are no
//!   windows.
//!
//! That combination still yields the `AssetPlugin`, `UiPlugin` and `TextPlugin`
//! (with its `CosmicFontSystem`) machinery, so `bevy_ui` computes a real layout
//! and the [`AssetServer`] resource is present — exactly what a UI-layout or
//! asset test needs, with no GPU and no window.
//!
//! Like [`GdtfTestAppBuilder`](crate::GdtfTestAppBuilder) this is a **type-state**
//! builder: [`build`](GdtfUiTestAppBuilder::build) is only reachable once a camera
//! has been provisioned (see [`with_ui_camera`](GdtfUiTestAppBuilder::with_ui_camera)),
//! because `bevy_ui` only computes a layout for a UI target that has a camera.
//! There is no fallible step — `build()` is infallible by construction, so the
//! harness never needs `unwrap`/`expect`/`panic`.
//!
//! The example below is `ignore`d by the doctest harness for the same
//! `dynamic_linking` dyld reason documented on
//! [`GdtfTestAppBuilder`](crate::GdtfTestAppBuilder); the behaviour it shows is
//! verified for real by this module's unit test.
//!
//! ```ignore
//! use bevy::prelude::*;
//! use gdtf_test_utils::GdtfUiTestAppBuilder;
//!
//! // `with_ui_camera()` spawns the harness's OWN Camera2d (distinct from any
//! // production camera) and unlocks `build()`.
//! let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
//!
//! let node = app.world_mut().spawn(Node::default()).id();
//! for _ in 0..3 {
//!     app.update();
//! }
//!
//! // The UI layout machinery ran: the node now carries a ComputedNode.
//! assert!(app.world().get::<ComputedNode>(node).is_some());
//! // The asset machinery is present.
//! assert!(app.world().get_resource::<AssetServer>().is_some());
//! ```

use core::marker::PhantomData;
use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    camera::Camera2d,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

/// Type-state marker: the builder has **not** yet been given a UI camera. In this
/// phase [`with_ui_camera`](GdtfUiTestAppBuilder::with_ui_camera) is available but
/// [`build`](GdtfUiTestAppBuilder::build) is not, because `bevy_ui` will not
/// compute a layout without a camera to target.
pub struct NoCamera;

/// Type-state marker: a UI camera has been provisioned, so
/// [`build`](GdtfUiTestAppBuilder::build) is now available.
pub struct WithCamera;

/// A type-state builder for a headless GDTF [`App`] that runs the real `bevy_ui`
/// layout and `bevy_asset` machinery **without a GPU or window**.
///
/// The `Phase` type parameter ([`NoCamera`] then [`WithCamera`]) tracks whether a
/// UI camera has been provisioned, which gates the API so that
/// [`build`](GdtfUiTestAppBuilder::build) cannot be called before a camera exists
/// — without a camera `bevy_ui` never computes a layout, so a cameraless app
/// would silently fail to be a UI harness at all. The `PhantomData` uses a
/// `fn() -> Phase` so the unused type parameter is covariant and carries no drop
/// or auto-trait obligations.
///
/// This is intentionally **separate** from the production [`crate::GdtfTestAppBuilder`]
/// (`MinimalPlugins`, no renderer/asset stack) and from any production camera
/// spawn system: the harness provisions its **own** [`Camera2d`] via
/// [`with_ui_camera`](Self::with_ui_camera) and never invokes a `gdtf_app` camera
/// system.
pub struct GdtfUiTestAppBuilder<Phase> {
    /// The headless app under construction.
    app:    App,
    /// Tracks the builder phase at the type level without storing a `Phase`.
    _phase: PhantomData<fn() -> Phase>,
}

/// Returns the workspace-root `assets/` directory as an absolute path.
///
/// Bevy's file [`AssetReader`](bevy::asset::io::AssetReader) resolves its base
/// path from `BEVY_ASSET_ROOT`, else `CARGO_MANIFEST_DIR`, else the executable's
/// directory (see `bevy_asset`'s `get_base_path`). Under `cargo test`,
/// `CARGO_MANIFEST_DIR` points at **this crate** (`crates/gdtf_test_utils`), so a
/// default `AssetPlugin` would look for assets under
/// `crates/gdtf_test_utils/assets`, not the repo-root `assets/` the running app
/// uses. To make the harness load loose assets from the **same** root as the app,
/// we compute the workspace root at compile time relative to this crate's
/// manifest (`crates/gdtf_test_utils` → up two levels) and join `assets` onto it,
/// then hand that absolute path to [`AssetPlugin::file_path`].
fn workspace_assets_root() -> PathBuf {
    // `env!("CARGO_MANIFEST_DIR")` is `<repo>/crates/gdtf_test_utils`; the
    // workspace root is two parents up. `Path::join("..")` keeps this purely
    // lexical (no filesystem access, no canonicalize that could fail), so it is
    // infallible and does not require the directory to exist yet.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

impl GdtfUiTestAppBuilder<NoCamera> {
    /// Builds a fresh headless app with the `no_renderer.rs` plugin set.
    ///
    /// Composes `DefaultPlugins`, then:
    /// 1. [`RenderPlugin`] with [`WgpuSettings`]`{ backends: None, .. }` so the
    ///    render app comes up with no GPU adapter (runnable on headless CI).
    /// 2. [`WinitPlugin`] disabled, so no OS event loop / window is created.
    /// 3. [`WindowPlugin`]`{ primary_window: None, exit_condition: DontExit, .. }`
    ///    so no window entity is spawned and the app does not exit on zero
    ///    windows.
    /// 4. [`AssetPlugin`] pointed at the **workspace-root** `assets/` directory
    ///    (see [`workspace_assets_root`]) so tests load loose assets from the same
    ///    root as the running app.
    ///
    /// The app has **no camera yet** — call [`with_ui_camera`](Self::with_ui_camera)
    /// to provision the harness's own [`Camera2d`] and reach [`WithCamera`], which
    /// unlocks [`build`](GdtfUiTestAppBuilder::build).
    #[must_use]
    pub fn new() -> Self {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        backends: None,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .disable::<WinitPlugin>()
                // Headless-test noise suppression (GTW-139): LogPlugin sets a
                // PROCESS-GLOBAL tracing subscriber (so the 2nd+ test app errors,
                // and it's the subscriber that prints every other bevy ERROR/WARN);
                // disabling it across the DefaultPlugins harnesses leaves no global
                // subscriber, silencing the otherwise-harmless headless logs
                // (incl. the `backends: None` RenderApp-absent ClearColor extract
                // ERROR). TerminalCtrlCHandler / Gizmo / Audio each probe a missing
                // window/RenderApp/device and are unused by a UI-layout test.
                // (`.disable` is a PluginGroupBuilder method, so it follows `.set`.)
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                .disable::<bevy::gizmos::GizmoPlugin>()
                .disable::<bevy::audio::AudioPlugin>()
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: workspace_assets_root().to_string_lossy().into_owned(),
                    ..default()
                }),
        );
        Self {
            app,
            _phase: PhantomData,
        }
    }

    /// Spawns the harness's **own** [`Camera2d`] and advances to [`WithCamera`].
    ///
    /// `bevy_ui` only computes a layout for UI nodes that have a camera to target,
    /// so this is required before [`build`](GdtfUiTestAppBuilder::build). The
    /// camera is spawned directly by the harness — it is **distinct** from any
    /// production camera and the harness never calls a `gdtf_app` camera system,
    /// keeping the test camera independent of production wiring.
    #[must_use]
    pub fn with_ui_camera(mut self) -> GdtfUiTestAppBuilder<WithCamera> {
        self.app.world_mut().spawn(Camera2d);
        GdtfUiTestAppBuilder {
            app:    self.app,
            _phase: PhantomData,
        }
    }
}

impl Default for GdtfUiTestAppBuilder<NoCamera> {
    fn default() -> Self {
        Self::new()
    }
}

impl GdtfUiTestAppBuilder<WithCamera> {
    /// Consumes the builder and returns the configured headless UI [`App`].
    ///
    /// This method exists **only** in the [`WithCamera`] phase, so it is a compile
    /// error to call `build()` before a UI camera has been provisioned via
    /// [`with_ui_camera`](GdtfUiTestAppBuilder::with_ui_camera).
    pub fn build(self) -> App {
        self.app
    }
}

#[cfg(test)]
mod tests {
    use bevy::{asset::AssetServer, prelude::Node, ui::ComputedNode};

    use super::*;

    #[test]
    fn ui_layout_runs_and_asset_server_present_headless() {
        let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();

        // A node with an explicit pixel size: once `bevy_ui` runs its layout, the
        // entity gains a `ComputedNode` whose `size()` resolves to this extent.
        // Without `UiPlugin` (i.e. if the headless render/ui stack were not wired)
        // no `ComputedNode` would ever appear, so this assertion fails if the
        // harness regresses to a non-UI plugin set — the pin-discriminating check.
        let node = app
            .world_mut()
            .spawn(Node {
                width: bevy::ui::Val::Px(120.0),
                height: bevy::ui::Val::Px(80.0),
                ..default()
            })
            .id();

        // A few frames so the asset + render schedules initialise and `bevy_ui`'s
        // layout system computes the node.
        for _ in 0..3 {
            app.update();
        }

        let computed = app.world().get::<ComputedNode>(node);
        assert!(
            computed.is_some(),
            "spawned Node should have received a ComputedNode — proves the bevy_ui \
             layout machinery ran under the headless harness",
        );

        if let Some(computed) = computed {
            let size = computed.size();
            assert!(
                (size.x - 120.0).abs() < 0.5 && (size.y - 80.0).abs() < 0.5,
                "ComputedNode size {size:?} should resolve to the node's explicit \
                 120x80 px extent — proves layout was actually computed, not just \
                 the component inserted",
            );
        }

        assert!(
            app.world().get_resource::<AssetServer>().is_some(),
            "AssetServer resource should be present — proves the asset machinery \
             is wired in the headless harness",
        );
    }
}
