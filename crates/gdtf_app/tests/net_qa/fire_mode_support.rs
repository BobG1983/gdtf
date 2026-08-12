//! Shared pieces for the cases that set a fire mode over the wire.

use std::sync::mpsc;

use bevy::{app::App, ecs::entity::Entity, prelude::*};
use gdtf_app::qa_wire::misc::ModeKindNet;
use gdtf_battle_input::SelectedFireMode;
use gdtf_battle_sim::weapon::{
    FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
};
use gdtf_net_qa_transport::IncomingRequest;
use gdtf_ui::{ActiveSegment, SegmentIndex};
use serde::Deserialize;

use super::battle_fixture::{
    arm_selected_with_modes, drive_into_battle_running, menu_app_with_net_qa,
};

/// What `battle.set_fire_mode` answers with.
#[derive(Debug, Deserialize)]
pub(crate) struct FireModeBody {
    pub(crate) mode: ModeKindNet,
}

/// The single these cases' gun offers, which is not what `SelectedFireMode` defaults to.
pub(crate) const SINGLE: FireModeSpec = FireModeSpec::new(
    ModeKind::Single,
    ModeConeMult::new(1.0),
    ModeTuPercent::new(0.5),
    ModeShots::new(1),
);

/// The burst that gun offers.
pub(crate) const BURST: FireModeSpec = FireModeSpec::new(
    ModeKind::Burst,
    ModeConeMult::new(1.5),
    ModeTuPercent::new(0.7),
    ModeShots::new(3),
);

/// The full-auto mode the three-mode gun offers on top of single and burst.
pub(crate) const FULL: FireModeSpec = FireModeSpec::new(
    ModeKind::Full,
    ModeConeMult::new(2.0),
    ModeTuPercent::new(0.9),
    ModeShots::new(6),
);

/// A gun that offers single and burst, which the shipped test weapon does not.
pub(crate) fn single_and_burst() -> FireMode {
    FireMode::new(vec![SINGLE, BURST])
}

/// A gun offering all three modes, so two writers can ask for different ones on one frame.
pub(crate) fn single_burst_and_full() -> FireMode {
    FireMode::new(vec![SINGLE, BURST, FULL])
}

/// The whole spec the world itself is holding, which a kind alone cannot tell apart.
pub(crate) fn selected_mode(app: &App) -> Option<FireModeSpec> {
    app.world()
        .get_resource::<SelectedFireMode>()
        .map(|mode| **mode)
}

/// The one mode segment carrying `M`, which a running battle always spawns.
pub(crate) fn mode_segment<M: Component>(app: &mut App) -> Entity {
    let mut query = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = query.iter(app.world()).collect();
    assert_eq!(
        found.len(),
        1,
        "a running battle spawns exactly one segment for each mode marker, and a case that \
         presses nothing proves nothing: {found:?}",
    );
    let Some(segment) = found.first().copied() else {
        unreachable!("the count of segments carrying the marker was just asserted to be one");
    };
    segment
}

/// Whether the mode panel is showing the segment carrying `M` as the active one.
pub(crate) fn mode_segment_is_active<M: Component>(app: &mut App) -> bool {
    let segment = mode_segment::<M>(app);
    let Some(index) = app
        .world()
        .get::<SegmentIndex>(segment)
        .map(|index| **index)
    else {
        return false;
    };
    let Some(control) = app.world().get::<ChildOf>(segment).map(ChildOf::parent) else {
        return false;
    };
    app.world()
        .get::<ActiveSegment>(control)
        .is_some_and(|active| **active == index)
}

/// The RON argument body that asks for `mode`.
pub(crate) fn mode_argument(mode: ModeKindNet) -> String {
    let Ok(text) = ron::ser::to_string(&mode) else {
        unreachable!("a wire fire-mode kind serializes to compact RON");
    };
    format!("(mode:{text})")
}

/// A battle whose selected shooter holds a gun offering single and burst, settled on single.
pub(crate) fn battle_with_a_burst_capable_gun() -> (App, mpsc::Sender<IncomingRequest>) {
    battle_with_a_gun(single_and_burst())
}

/// A battle whose selected shooter holds `modes`, settled on that gun's own single.
pub(crate) fn battle_with_a_gun(modes: FireMode) -> (App, mpsc::Sender<IncomingRequest>) {
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    let _weapon = arm_selected_with_modes(&mut app, modes);
    app.update();
    assert_eq!(
        selected_mode(&app),
        Some(SINGLE),
        "the frame after arming lets the selection sync settle, so the case starts on the gun's \
         own single spec — which is not the resource's default — and a later mode cannot be the \
         value it was already holding",
    );
    (app, tx)
}
