use gdtf_qa_protocol::command::{FALLBACK_SHAPE_TEXT, RonShape, ShapeDoc, ShapeName, shape_trace};
use serde::de::DeserializeOwned;

use crate::dev::net_qa::{
    commands::{
        read::app_phase::{AppPhaseArgs, AppPhaseReply},
        set::GAME_COMMANDS,
    },
    wire::{
        act::{ActSeqNet, NetIntent},
        act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
        cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
        key::{FocusStepNet, KeyNet, KeyPressNet, KeybindActionNet},
        log::{ActProvenanceNet, LogDroppedCount, LogReadCap},
        misc::{
            AutoRunNet, FireModeIndex, FrameDelay, RequestId, SeedNet, SituationRef,
            StepperCommandNet,
        },
        phase::{
            AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
            RunningPhaseNet,
        },
        pointer::{MouseButtonNet, PointerPosNet, PointerXNet, PointerYNet},
        shell::{CaughtUpNet, SoundNet},
        token::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
    },
};

fn named_root_resolves(doc: &ShapeDoc, named: &str) {
    let RonShape::Named(name) = doc.root() else {
        return;
    };
    assert!(
        doc.body_of(name).is_some(),
        "`{named}`'s shape references `{}` as its root but defines no body for it: {doc:?}",
        name.as_str(),
    );
}

fn assert_schema_is_usable<T: DeserializeOwned>(named: &str) {
    match shape_trace::<T>() {
        Ok(doc) => named_root_resolves(&doc, named),
        Err(fault) => {
            unreachable!("`{named}`'s shape must trace out of its own Deserialize impl: {fault}")
        }
    }
}

fn parsed(document: &str, named: &str) -> ShapeDoc {
    let Ok(doc) = ron::de::from_str::<ShapeDoc>(document) else {
        unreachable!("`{named}` publishes a parseable shape, got `{document}`");
    };
    doc
}

#[test]
fn every_wire_type_traces_a_usable_shape() {
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

    assert_schema_is_usable::<SoundNet>("SoundNet");
    assert_schema_is_usable::<CaughtUpNet>("CaughtUpNet");

    assert_schema_is_usable::<LifecyclePhaseNet>("LifecyclePhaseNet");
    assert_schema_is_usable::<RunningPhaseNet>("RunningPhaseNet");
    assert_schema_is_usable::<GamePhaseNet>("GamePhaseNet");
    assert_schema_is_usable::<BattleScapePhaseNet>("BattleScapePhaseNet");
    assert_schema_is_usable::<AfterMathPhaseNet>("AfterMathPhaseNet");
    assert_schema_is_usable::<AppPhaseNet>("AppPhaseNet");
}

#[test]
fn every_registered_command_traces_both_of_its_own_types() {
    assert!(
        shape_trace::<AppPhaseArgs>().is_ok(),
        "app.phase's argument type traces",
    );
    assert!(
        shape_trace::<AppPhaseReply>().is_ok(),
        "app.phase's reply type traces",
    );

    for command in GAME_COMMANDS {
        let name = command.name();
        for (side, document) in [
            ("argument", command.arg_schema().as_str().to_owned()),
            ("reply", command.reply_schema().as_str().to_owned()),
        ] {
            assert_ne!(
                document,
                FALLBACK_SHAPE_TEXT,
                "`{}`'s {side} type did not trace — it published the fallback",
                name.as_str(),
            );
            let doc = parsed(&document, name.as_str());
            named_root_resolves(&doc, name.as_str());
        }
    }
}

#[test]
fn a_named_root_without_a_body_is_caught() {
    let doc = ShapeDoc::new(
        RonShape::Named(ShapeName::from_static("Missing")),
        Vec::new(),
    );
    let outcome = std::panic::catch_unwind(|| named_root_resolves(&doc, "Missing"));
    assert!(
        outcome.is_err(),
        "the usable check must reject a root that names a type the document never defines",
    );
}
