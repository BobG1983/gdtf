use bevy::{app::App, ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter, dispatch_act_intents};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::acts::{
    EndTurnRequested, FireRequested, MoveRequested, ReloadRequested, SetAimingRequested,
    SetFacingRequested, SetStanceRequested,
};

pub(crate) fn install_intent_drain(app: &mut App) {
    app.init_resource::<PendingActIntent>()
        .init_resource::<SelectedShooter>()
        .init_resource::<ActiveLevel>()
        .init_resource::<ViewMode>()
        .add_message::<FireRequested>()
        .add_message::<MoveRequested>()
        .add_message::<SetStanceRequested>()
        .add_message::<SetAimingRequested>()
        .add_message::<SetFacingRequested>()
        .add_message::<ReloadRequested>()
        .add_message::<EndTurnRequested>();
}

pub(crate) fn push(app: &mut App, intent: ActIntent) {
    let Some(mut pending) = app.world_mut().get_resource_mut::<PendingActIntent>() else {
        unreachable!("the fixture installs the intent queue");
    };
    pending.push(intent);
}

pub(crate) fn push_stance_cycle(app: &mut App) {
    push(app, ActIntent::StanceCycle);
}

pub(crate) fn push_level_up(app: &mut App) {
    push(app, ActIntent::LevelUp);
}

pub(crate) fn push_toggle_full_view(app: &mut App) {
    push(app, ActIntent::ToggleFullView);
}

pub(crate) fn select(app: &mut App, actor: Entity) {
    app.insert_resource(SelectedShooter::new(actor));
}

pub(crate) fn drain(app: &mut App) {
    let ran = app.world_mut().run_system_once(dispatch_act_intents);
    assert!(ran.is_ok(), "the intent drain must run in the fixture");
}

pub(crate) fn drained<M: Message>(app: &mut App) -> usize {
    app.world_mut()
        .get_resource_mut::<Messages<M>>()
        .map_or(0, |mut messages| messages.drain().count())
}
