//! Every command name the game publishes, in declaration order.

use gdtf_qa_protocol::command::CommandName;

pub(crate) const APP_PHASE: &str = "app.phase";

pub(crate) const CAPTURE_SCREENSHOT: &str = "capture.screenshot";

pub(crate) const SETTINGS_READ: &str = "settings.read";

pub(crate) const UI_FOCUS: &str = "ui.focus";

pub(crate) const PLAYBACK_STATE: &str = "playback.state";

pub(crate) const BATTLE_ROSTER: &str = "battle.roster";

pub(crate) const BATTLE_TURN: &str = "battle.turn";

pub(crate) const BATTLE_SELECTION: &str = "battle.selection";

pub(crate) const BATTLE_OFFERS: &str = "battle.offers";

pub(crate) const BATTLE_INSPECT: &str = "battle.inspect";

pub(crate) const BATTLE_SIGHTLINE: &str = "battle.sightline";

pub(crate) const BATTLE_VISIBLE: &str = "battle.visible";

pub(crate) const BATTLE_REACHABLE: &str = "battle.reachable";

pub(crate) const BATTLE_COST: &str = "battle.cost";

pub(crate) const LOG_READ: &str = "log.read";

pub(crate) const LOG_OMNISCIENT_READ: &str = "log.omniscient_read";

pub(crate) const BATTLE_START: &str = "battle.start";

pub(crate) const BATTLE_FLEE: &str = "battle.flee";

pub(crate) const PROCGEN_STEP: &str = "procgen.step";

pub(crate) const WAIT: &str = "wait";

pub(crate) const ACT_SELECT: &str = "act.select";

pub(crate) const ACT_SELECT_NEXT: &str = "act.select_next";

pub(crate) const ACT_SELECT_PREV: &str = "act.select_prev";

pub(crate) const ACT_SELECT_CLEAR: &str = "act.select_clear";

pub(crate) const ACT_MOVE: &str = "act.move";

pub(crate) const ACT_FIRE: &str = "act.fire";

pub(crate) const ACT_RELOAD: &str = "act.reload";

pub(crate) const ACT_SET_STANCE: &str = "act.set_stance";

pub(crate) const ACT_SET_AIMING: &str = "act.set_aiming";

pub(crate) const ACT_SET_FACING: &str = "act.set_facing";

pub(crate) const ACT_END_TURN: &str = "act.end_turn";

pub(crate) const ACT_MELEE: &str = "act.melee";

pub(crate) const ACT_SHOVE: &str = "act.shove";

pub(crate) const ACT_STABILIZE: &str = "act.stabilize";

pub(crate) const ACT_EXECUTE: &str = "act.execute";

pub(crate) const ACT_THROW_GRENADE: &str = "act.throw_grenade";

pub(crate) const ACT_OPEN_DOOR: &str = "act.open_door";

pub(crate) const ACT_ENTER_EMPLACEMENT: &str = "act.enter_emplacement";

pub(crate) const ACT_EXIT_EMPLACEMENT: &str = "act.exit_emplacement";

pub(crate) const INPUT_PRESS_KEY: &str = "input.press_key";

pub(crate) const INPUT_HOVER: &str = "input.hover";

pub(crate) const INPUT_SET_FOCUS: &str = "input.set_focus";

pub(crate) const INPUT_FOCUS_STEP: &str = "input.focus_step";

pub(crate) const INPUT_ACTIVATE: &str = "input.activate";

pub(crate) const INPUT_CLICK_CELL: &str = "input.click_cell";

pub(crate) const VIEW_LEVEL_UP: &str = "view.level_up";

pub(crate) const VIEW_LEVEL_DOWN: &str = "view.level_down";

pub(crate) const VIEW_TOGGLE_FULL_VIEW: &str = "view.toggle_full_view";

pub(crate) const VIEW_PAN: &str = "view.pan";

pub(crate) const VIEW_LOOK_AT: &str = "view.look_at";

pub(crate) const BATTLE_SET_FIRE_MODE: &str = "battle.set_fire_mode";

/// Every command the game publishes, in declaration order.
pub(crate) fn published_names() -> Vec<CommandName> {
    [
        APP_PHASE,
        CAPTURE_SCREENSHOT,
        SETTINGS_READ,
        UI_FOCUS,
        PLAYBACK_STATE,
        BATTLE_ROSTER,
        BATTLE_TURN,
        BATTLE_SELECTION,
        BATTLE_OFFERS,
        BATTLE_INSPECT,
        BATTLE_SIGHTLINE,
        BATTLE_VISIBLE,
        BATTLE_REACHABLE,
        BATTLE_COST,
        LOG_READ,
        LOG_OMNISCIENT_READ,
        BATTLE_START,
        BATTLE_FLEE,
        PROCGEN_STEP,
        WAIT,
        ACT_SELECT,
        ACT_SELECT_NEXT,
        ACT_SELECT_PREV,
        ACT_SELECT_CLEAR,
        ACT_MOVE,
        ACT_FIRE,
        ACT_RELOAD,
        ACT_SET_STANCE,
        ACT_SET_AIMING,
        ACT_SET_FACING,
        ACT_END_TURN,
        ACT_MELEE,
        ACT_SHOVE,
        ACT_STABILIZE,
        ACT_EXECUTE,
        ACT_THROW_GRENADE,
        ACT_OPEN_DOOR,
        ACT_ENTER_EMPLACEMENT,
        ACT_EXIT_EMPLACEMENT,
        INPUT_PRESS_KEY,
        INPUT_HOVER,
        INPUT_SET_FOCUS,
        INPUT_FOCUS_STEP,
        INPUT_ACTIVATE,
        INPUT_CLICK_CELL,
        VIEW_LEVEL_UP,
        VIEW_LEVEL_DOWN,
        VIEW_TOGGLE_FULL_VIEW,
        VIEW_PAN,
        VIEW_LOOK_AT,
        BATTLE_SET_FIRE_MODE,
    ]
    .into_iter()
    .map(CommandName::from_static)
    .collect()
}
