//! Systems for the battlescape contextual panel (GTW-294) — spawn / despawn the bottom-right
//! contextual cluster on the `BattleScapeState::BattleRunning` boundary, plus the live slice's
//! detection (`detect_contextual_targets`) + press routing (`contextual_button_intents`).

mod spawn;

pub(in crate::scenes::running::game::battlescape) use spawn::{
    despawn_contextual_panel, spawn_contextual_panel,
};

mod detect;

pub(in crate::scenes::running::game::battlescape) use detect::detect_contextual_targets;

mod intents;

pub(in crate::scenes::running::game::battlescape) use intents::contextual_button_intents;
