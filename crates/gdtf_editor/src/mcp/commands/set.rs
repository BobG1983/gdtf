//! The one list of commands the editor host publishes.

use cobalt_mcp_host::command::ErasedCommand;

use super::{
    capture::EditorCaptureScreenshot,
    read::{
        EditorDraft, EditorFamilies, EditorLastSave, EditorPaintedMap, EditorPhase, EditorSession,
        EditorValidation, EditorWeighting,
    },
    wait::EditorWait,
    write::{
        EditorDeleteRecord, EditorListOp, EditorLoadArmor, EditorLoadAttachment, EditorLoadField,
        EditorLoadGang, EditorLoadInjury, EditorLoadMeleeWeapon, EditorLoadPrefab,
        EditorLoadSprite, EditorLoadTerrain, EditorLoadTheme, EditorLoadWeapon, EditorNew,
        EditorPaint, EditorSave, EditorSaveWeighting, EditorSelectFacing, EditorSelectInjuryTab,
        EditorSelectTheme, EditorSelectTile, EditorSelectWeightingTable, EditorSetDefaultFloor,
        EditorSetField, EditorSetGridSize, EditorSetLevel, EditorSetMode, EditorToggleTerrain,
    },
};
use crate::mcp::facts::EditorFacts;

pub(in crate::mcp) const EDITOR_COMMANDS: &[&dyn ErasedCommand<EditorFacts>] = &[
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
    &EditorLoadPrefab,
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
    &EditorDeleteRecord,
];
