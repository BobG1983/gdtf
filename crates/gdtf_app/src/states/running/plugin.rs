use bevy::prelude::*;

use crate::states::{
    AppState, RunningState,
    running::{
        GameScenePlugin, GangEditorScenePlugin, MenuScenePlugin, OptionsScenePlugin,
        QuitScenePlugin, systems::*,
    },
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

pub(in crate::states) struct RunningScenePlugin;

impl Plugin for RunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    // The persistent UI camera is spawned on entry to `AppState::Running` and is
    // owned by the app shell (this scene plugin), not by `gdtf_ui`: its lifetime
    // is tied to the `Running` phase as a whole — every `RunningState` screen
    // renders against it — rather than to any one sub-state or widget. It carries
    // no scene-scoped despawn marker, so it outlives every `RunningState`
    // transition.
    let label = SceneLabel::new("Running");
    app.add_systems(
        OnEnter(AppState::Running),
        (log_scene_enter(label), spawn_ui_camera),
    )
    .add_systems(OnExit(AppState::Running), log_scene_exit(label));
}

fn add_plugins(app: &mut App) {
    app.add_plugins(MenuScenePlugin)
        .add_plugins(GameScenePlugin)
        .add_plugins(OptionsScenePlugin)
        .add_plugins(QuitScenePlugin)
        // The DEV-ONLY gang editor (GTW-420). Its scene plugin is always registered (the
        // `DebugGangEditor` state variant always exists), but the only entry point — the
        // `cfg(debug_assertions)`-gated "Gang Editor" menu button — never compiles into a
        // release binary, so the editor is unreachable in release.
        .add_plugins(GangEditorScenePlugin);

    // The DEV-ONLY procgen STEP/AUTO visualizer (GTW-434). Its whole module — including this
    // plugin — is `#[cfg(debug_assertions)]`-gated, so a release build neither registers nor
    // compiles it (C4). Like the gang editor, its only entry point is the
    // `cfg(debug_assertions)`-gated "Procgen Viz" menu button.
    #[cfg(debug_assertions)]
    app.add_plugins(crate::states::running::ProcgenVizScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<RunningState>();
}
