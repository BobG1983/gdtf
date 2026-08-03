use schemars::{JsonSchema, schema_for};
use serde_json::{Map, Value};

use crate::dev::net_qa::wire::{
    act::{ActSeqNet, NetIntent},
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    key::{FocusStepNet, KeyNet, KeyPressNet, KeybindActionNet},
    log::{ActProvenanceNet, LogDroppedCount, LogReadCap},
    misc::{
        AutoRunNet, FireModeIndex, FrameDelay, RequestId, SeedNet, SituationRef, StepperCommandNet,
    },
    phase::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    },
    pointer::{MouseButtonNet, PointerPosNet, PointerXNet, PointerYNet},
    token::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
};

fn schema_object<T: JsonSchema>(named: &str) -> Map<String, Value> {
    let Ok(text) = serde_json::to_string(&schema_for!(T)) else {
        unreachable!("`{named}`'s derived schema serializes to JSON text");
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
        unreachable!("`{named}`'s derived schema is parseable JSON, got `{text}`");
    };
    let Value::Object(object) = parsed else {
        unreachable!("`{named}`'s derived schema is a JSON object, got `{text}`");
    };
    object
}

fn assert_schema_is_usable<T: JsonSchema>(named: &str) {
    let object = schema_object::<T>(named);
    assert!(
        object.contains_key("type")
            || object.contains_key("oneOf")
            || object.contains_key("anyOf")
            || object.contains_key("$ref"),
        "`{named}`'s derived schema describes a shape, got `{object:?}`",
    );
}

fn assert_denies_unknown_fields<T: JsonSchema>(named: &str) {
    let object = schema_object::<T>(named);
    assert_eq!(
        object.get("type").and_then(Value::as_str),
        Some("object"),
        "`{named}` is a named-field struct, so its schema describes an object: `{object:?}`",
    );
    assert_eq!(
        object.get("additionalProperties"),
        Some(&Value::Bool(false)),
        "`{named}`'s schema refuses unknown properties: `{object:?}`",
    );
}

#[test]
fn every_wire_type_derives_a_usable_schema() {
    assert_schema_is_usable::<CellXNet>("CellXNet");
    assert_schema_is_usable::<CellYNet>("CellYNet");
    assert_schema_is_usable::<LevelNet>("LevelNet");
    assert_schema_is_usable::<CellNet>("CellNet");
    assert_schema_is_usable::<CellLevelNet>("CellLevelNet");

    assert_schema_is_usable::<GangerToken>("GangerToken");
    assert_schema_is_usable::<DoorToken>("DoorToken");
    assert_schema_is_usable::<EmplacementToken>("EmplacementToken");
    assert_schema_is_usable::<FocusTargetNet>("FocusTargetNet");

    assert_schema_is_usable::<PointerXNet>("PointerXNet");
    assert_schema_is_usable::<PointerYNet>("PointerYNet");
    assert_schema_is_usable::<PointerPosNet>("PointerPosNet");
    assert_schema_is_usable::<MouseButtonNet>("MouseButtonNet");

    assert_schema_is_usable::<NetIntent>("NetIntent");
    assert_schema_is_usable::<ActSeqNet>("ActSeqNet");
    assert_schema_is_usable::<StanceNet>("StanceNet");
    assert_schema_is_usable::<AimNet>("AimNet");
    assert_schema_is_usable::<FacingNet>("FacingNet");
    assert_schema_is_usable::<MeleeTargetNet>("MeleeTargetNet");

    assert_schema_is_usable::<KeyNet>("KeyNet");
    assert_schema_is_usable::<KeybindActionNet>("KeybindActionNet");
    assert_schema_is_usable::<KeyPressNet>("KeyPressNet");
    assert_schema_is_usable::<FocusStepNet>("FocusStepNet");

    assert_schema_is_usable::<ActProvenanceNet>("ActProvenanceNet");
    assert_schema_is_usable::<LogReadCap>("LogReadCap");
    assert_schema_is_usable::<LogDroppedCount>("LogDroppedCount");

    assert_schema_is_usable::<FireModeIndex>("FireModeIndex");
    assert_schema_is_usable::<SituationRef>("SituationRef");
    assert_schema_is_usable::<SeedNet>("SeedNet");
    assert_schema_is_usable::<FrameDelay>("FrameDelay");
    assert_schema_is_usable::<RequestId>("RequestId");
    assert_schema_is_usable::<AutoRunNet>("AutoRunNet");
    assert_schema_is_usable::<StepperCommandNet>("StepperCommandNet");

    assert_schema_is_usable::<LifecyclePhaseNet>("LifecyclePhaseNet");
    assert_schema_is_usable::<RunningPhaseNet>("RunningPhaseNet");
    assert_schema_is_usable::<GamePhaseNet>("GamePhaseNet");
    assert_schema_is_usable::<BattleScapePhaseNet>("BattleScapePhaseNet");
    assert_schema_is_usable::<AfterMathPhaseNet>("AfterMathPhaseNet");
    assert_schema_is_usable::<AppPhaseNet>("AppPhaseNet");
}

#[test]
fn every_named_field_struct_refuses_unknown_properties() {
    assert_denies_unknown_fields::<CellNet>("CellNet");
    assert_denies_unknown_fields::<CellLevelNet>("CellLevelNet");
    assert_denies_unknown_fields::<PointerPosNet>("PointerPosNet");
    assert_denies_unknown_fields::<AppPhaseNet>("AppPhaseNet");
}
