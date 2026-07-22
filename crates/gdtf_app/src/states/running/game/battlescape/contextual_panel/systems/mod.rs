//! Systems for the battlescape contextual panel (GTW-294 / GTW-571) — the root box
//! spawn / despawn on the `BattleScapeState::BattleRunning` boundary, plus the GENERIC
//! per-act button machinery (spawn / visibility toggle / press router) and the two
//! act-agnostic panel passes (deterministic child ordering + root visibility).

mod spawn;

pub(in crate::states::running::game::battlescape) use spawn::{
    despawn_contextual_panel, spawn_contextual_panel,
};

mod buttons;

pub(in crate::states::running::game::battlescape) use buttons::{
    order_contextual_buttons, press_contextual_button, spawn_contextual_button,
    sync_contextual_button_visibility, sync_panel_root_visibility,
};

mod slot_keys;

pub(in crate::states::running::game::battlescape) use slot_keys::{
    press_contextual_button_via_key, rank_visible_contextual_buttons,
};
