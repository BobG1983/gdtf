use bevy::{prelude::*, state::state::FreelyMutableState};

use super::SceneLabel;

pub(in crate::states) fn log_scene_enter(label: SceneLabel) -> impl Fn() {
    let message = format!("Entered {} State", *label);
    move || info!("{message}")
}

pub(in crate::states) fn log_scene_exit(label: SceneLabel) -> impl Fn() {
    let message = format!("Exiting {} State", *label);
    move || info!("{message}")
}

pub(in crate::states) fn insert_completion_marker<M: Resource + Default>() -> impl Fn(Commands) {
    |mut commands: Commands| commands.insert_resource(M::default())
}

pub(in crate::states) fn remove_scoped_resource<R: Resource>() -> impl Fn(Commands) {
    |mut commands: Commands| commands.remove_resource::<R>()
}

pub(in crate::states) fn advance_state_to<S: FreelyMutableState>(
    target: S,
) -> impl Fn(ResMut<NextState<S>>) {
    move |mut next: ResMut<NextState<S>>| next.set(target.clone())
}
