//! The one list of commands the editor host publishes.

use gdtf_qa_command::command::ErasedCommand;

use super::{
    capture::EditorCaptureScreenshot,
    read::{
        EditorDraft, EditorFamilies, EditorLastSave, EditorPaintedMap, EditorPhase, EditorSession,
        EditorValidation, EditorWeighting,
    },
    wait::EditorWait,
    write::{
        EditorListOp, EditorLoadArmor, EditorLoadAttachment, EditorLoadField, EditorLoadGang,
        EditorLoadInjury, EditorLoadMeleeWeapon, EditorLoadSprite, EditorLoadTerrain,
        EditorLoadTheme, EditorLoadWeapon, EditorNew, EditorPaint, EditorSave, EditorSaveWeighting,
        EditorSelectFacing, EditorSelectInjuryTab, EditorSelectTheme, EditorSelectTile,
        EditorSelectWeightingTable, EditorSetDefaultFloor, EditorSetField, EditorSetGridSize,
        EditorSetLevel, EditorSetMode, EditorToggleTerrain,
    },
};
use crate::net_qa::facts::EditorFacts;

pub(in crate::net_qa) const EDITOR_COMMANDS: &[&dyn ErasedCommand<EditorFacts>] = &[
    &EditorPhase,
    &EditorLastSave,
    &EditorValidation,
    &EditorFamilies,
    &EditorSession,
    &EditorDraft,
    &EditorSetMode,
    &EditorNew,
    &EditorLoadTheme,
    &EditorLoadGang,
    &EditorLoadArmor,
    &EditorLoadInjury,
    &EditorLoadSprite,
    &EditorLoadAttachment,
    &EditorLoadWeapon,
    &EditorLoadMeleeWeapon,
    &EditorLoadField,
    &EditorLoadTerrain,
    &EditorSave,
    &EditorSetField,
    &EditorListOp,
    &EditorSelectTheme,
    &EditorToggleTerrain,
    &EditorSetDefaultFloor,
    &EditorPaintedMap,
    &EditorSetGridSize,
    &EditorSelectTile,
    &EditorSelectFacing,
    &EditorSetLevel,
    &EditorPaint,
    &EditorSelectInjuryTab,
    &EditorSelectWeightingTable,
    &EditorWeighting,
    &EditorSaveWeighting,
    &EditorCaptureScreenshot,
    &EditorWait,
];
