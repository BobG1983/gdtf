use gdtf_qa_command::command::ErasedCommand;
use gdtf_qa_protocol::message::ServerNameNet;

use super::{
    act::{
        ActEndTurn, ActEnterEmplacement, ActExecute, ActExitEmplacement, ActFire, ActMelee,
        ActMove, ActOpenDoor, ActReload, ActSelect, ActSelectClear, ActSelectNext, ActSelectPrev,
        ActSetAiming, ActSetFacing, ActSetStance, ActShove, ActStabilize, ActThrowGrenade,
    },
    capture::CaptureScreenshot,
    control::{
        BattleSetFireMode, ViewLevelDown, ViewLevelUp, ViewLookAt, ViewPan, ViewToggleFullView,
    },
    input::{
        InputActivate, InputClickCell, InputFocusStep, InputHover, InputPressKey, InputSetFocus,
    },
    lifecycle::{BattleFlee, BattleStart},
    procgen::ProcgenStep,
    read::{
        AppPhase, BattleCost, BattleInspect, BattleOffers, BattleReachable, BattleRoster,
        BattleSelection, BattleSightline, BattleTurn, BattleVisible, LogOmniscientRead, LogRead,
        PlaybackState, SettingsRead, UiFocus,
    },
    wait::Wait,
};
use crate::dev::net_qa::{config::SERVER_NAME, facts::GameFacts};

pub(in crate::dev::net_qa) const GAME_COMMANDS: &[&dyn ErasedCommand<GameFacts>] = &[
    &AppPhase,
    &CaptureScreenshot,
    &SettingsRead,
    &UiFocus,
    &PlaybackState,
    &BattleRoster,
    &BattleTurn,
    &BattleSelection,
    &BattleOffers,
    &BattleInspect,
    &BattleSightline,
    &BattleVisible,
    &BattleReachable,
    &BattleCost,
    &LogRead,
    &LogOmniscientRead,
    &BattleStart,
    &BattleFlee,
    &ProcgenStep,
    &Wait,
    &ActSelect,
    &ActSelectNext,
    &ActSelectPrev,
    &ActSelectClear,
    &ActMove,
    &ActFire,
    &ActReload,
    &ActSetStance,
    &ActSetAiming,
    &ActSetFacing,
    &ActEndTurn,
    &ActMelee,
    &ActShove,
    &ActStabilize,
    &ActExecute,
    &ActThrowGrenade,
    &ActOpenDoor,
    &ActEnterEmplacement,
    &ActExitEmplacement,
    &InputPressKey,
    &InputHover,
    &InputSetFocus,
    &InputFocusStep,
    &InputActivate,
    &InputClickCell,
    &ViewLevelUp,
    &ViewLevelDown,
    &ViewToggleFullView,
    &ViewPan,
    &ViewLookAt,
    &BattleSetFireMode,
];

#[must_use]
pub(in crate::dev::net_qa) fn game_host_name() -> ServerNameNet {
    ServerNameNet::new(SERVER_NAME.to_owned())
}
