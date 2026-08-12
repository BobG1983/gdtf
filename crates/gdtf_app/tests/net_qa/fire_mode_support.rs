//! Shared pieces for the cases that set a fire mode over the wire.

use std::sync::mpsc;

use bevy::app::App;
use gdtf_app::qa_wire::misc::ModeKindNet;
use gdtf_battle_input::SelectedFireMode;
use gdtf_battle_sim::weapon::{
    FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
};
use gdtf_net_qa_transport::IncomingRequest;
use serde::Deserialize;

use super::battle_fixture::{
    arm_selected_with_modes, drive_into_battle_running, menu_app_with_net_qa,
};

/// What `battle.set_fire_mode` answers with.
#[derive(Debug, Deserialize)]
pub(crate) struct FireModeBody {
    pub(crate) mode: ModeKindNet,
}

/// A gun that offers single and burst, which the shipped test weapon does not.
pub(crate) fn single_and_burst() -> FireMode {
    FireMode::new(vec![
        FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        ),
        FireModeSpec::new(
            ModeKind::Burst,
            ModeConeMult::new(1.5),
            ModeTuPercent::new(0.7),
            ModeShots::new(3),
        ),
    ])
}

/// The mode the world itself is holding.
pub(crate) fn selected_mode(app: &App) -> ModeKindNet {
    let Some(mode) = app.world().get_resource::<SelectedFireMode>() else {
        unreachable!("the input plugin inits SelectedFireMode when it is built");
    };
    ModeKindNet::from_sim(mode.kind)
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
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    let _weapon = arm_selected_with_modes(&mut app, single_and_burst());
    app.update();
    assert_eq!(
        selected_mode(&app),
        ModeKindNet::Single,
        "the frame after arming lets the selection sync settle, so the case starts on single and \
         a later Burst cannot be the value it was already holding",
    );
    (app, tx)
}
