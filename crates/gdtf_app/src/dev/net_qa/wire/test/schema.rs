use gdtf_qa_protocol::command::{
    FALLBACK_SHAPE_TEXT, RonShape, ShapeBody, ShapeDoc, ShapeName, shape_trace,
};
use serde::de::DeserializeOwned;

use crate::dev::net_qa::{
    commands::{
        read::app_phase::{AppPhaseArgs, AppPhaseReply},
        set::GAME_COMMANDS,
    },
    wire::{
        act::{ActCompleteNet, ActRefusalNet, ActReply, ActSeqNet, NetIntent, SelectReply},
        act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
        cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
        cost::{CostActNet, CostLegalNet, CostRefusalNet},
        deed::{ActDeedKindNet, MoveRejectionNet},
        inspect::{
            CoverBlockNet, CoverHpNet, HardnessNet, HeightBandNet, InspectShownNet, ProtectionNet,
        },
        key::{FocusStepNet, KeyNet, KeyPressNet, KeybindActionNet},
        log::{ActProvenanceNet, LogDroppedCount, LogEntryNet, LogReadCap},
        misc::{
            AutoRunNet, FireModeIndex, FrameDelay, ModeKindNet, ProcgenStageNet, RequestId,
            SeedNet, SituationRef, StepperCommandNet, ViewModeNet,
        },
        offer::{ContextualActNet, ContextualOfferNet, OfferPressableNet, OfferTargetNet},
        phase::{
            AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
            RunningPhaseNet,
        },
        pointer::{MouseButtonNet, PointerPosNet, PointerXNet, PointerYNet},
        roster::{FactionNet, GangerCardNet, GangerNameNet},
        shell::{CaughtUpNet, SoundNet},
        sight::{CanEngageNet, CanSeeNet, SightlineNet},
        token::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
        visible::{DoorOpenNet, VisibleCoverNet, VisibleDoorNet, VisibleGangerNet},
        vitals::{HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet},
        wait::{ActCountNet, AppPhaseTargetNet, WaitConditionNet},
        wound::{BodyPartNet, InjuryNameNet, InjuryNet, SeverityNet, WoundNet},
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
    assert_schema_is_usable::<ActCompleteNet>("ActCompleteNet");
    assert_schema_is_usable::<ActRefusalNet>("ActRefusalNet");
    assert_schema_is_usable::<ActReply>("ActReply");
    assert_schema_is_usable::<SelectReply>("SelectReply");
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
    assert_schema_is_usable::<MoveRejectionNet>("MoveRejectionNet");
    assert_schema_is_usable::<ActDeedKindNet>("ActDeedKindNet");
    assert_schema_is_usable::<LogEntryNet>("LogEntryNet");

    assert_schema_is_usable::<TuNet>("TuNet");
    assert_schema_is_usable::<TuMaxNet>("TuMaxNet");
    assert_schema_is_usable::<HpNet>("HpNet");
    assert_schema_is_usable::<HpMaxNet>("HpMaxNet");
    assert_schema_is_usable::<WoundsNet>("WoundsNet");
    assert_schema_is_usable::<WoundsMaxNet>("WoundsMaxNet");

    assert_schema_is_usable::<SeverityNet>("SeverityNet");
    assert_schema_is_usable::<BodyPartNet>("BodyPartNet");
    assert_schema_is_usable::<WoundNet>("WoundNet");
    assert_schema_is_usable::<InjuryNameNet>("InjuryNameNet");
    assert_schema_is_usable::<InjuryNet>("InjuryNet");

    assert_schema_is_usable::<GangerNameNet>("GangerNameNet");
    assert_schema_is_usable::<FactionNet>("FactionNet");
    assert_schema_is_usable::<GangerCardNet>("GangerCardNet");

    assert_schema_is_usable::<HardnessNet>("HardnessNet");
    assert_schema_is_usable::<ProtectionNet>("ProtectionNet");
    assert_schema_is_usable::<HeightBandNet>("HeightBandNet");
    assert_schema_is_usable::<CoverHpNet>("CoverHpNet");
    assert_schema_is_usable::<CoverBlockNet>("CoverBlockNet");
    assert_schema_is_usable::<InspectShownNet>("InspectShownNet");

    assert_schema_is_usable::<DoorOpenNet>("DoorOpenNet");
    assert_schema_is_usable::<VisibleGangerNet>("VisibleGangerNet");
    assert_schema_is_usable::<VisibleDoorNet>("VisibleDoorNet");
    assert_schema_is_usable::<VisibleCoverNet>("VisibleCoverNet");

    assert_schema_is_usable::<ContextualActNet>("ContextualActNet");
    assert_schema_is_usable::<OfferTargetNet>("OfferTargetNet");
    assert_schema_is_usable::<OfferPressableNet>("OfferPressableNet");
    assert_schema_is_usable::<ContextualOfferNet>("ContextualOfferNet");

    assert_schema_is_usable::<CanSeeNet>("CanSeeNet");
    assert_schema_is_usable::<CanEngageNet>("CanEngageNet");
    assert_schema_is_usable::<SightlineNet>("SightlineNet");

    assert_schema_is_usable::<ModeKindNet>("ModeKindNet");
    assert_schema_is_usable::<ViewModeNet>("ViewModeNet");

    assert_schema_is_usable::<CostActNet>("CostActNet");
    assert_schema_is_usable::<CostLegalNet>("CostLegalNet");
    assert_schema_is_usable::<CostRefusalNet>("CostRefusalNet");

    assert_schema_is_usable::<FireModeIndex>("FireModeIndex");
    assert_schema_is_usable::<SituationRef>("SituationRef");
    assert_schema_is_usable::<SeedNet>("SeedNet");
    assert_schema_is_usable::<FrameDelay>("FrameDelay");
    assert_schema_is_usable::<RequestId>("RequestId");
    assert_schema_is_usable::<AutoRunNet>("AutoRunNet");
    assert_schema_is_usable::<StepperCommandNet>("StepperCommandNet");
    assert_schema_is_usable::<ProcgenStageNet>("ProcgenStageNet");

    assert_schema_is_usable::<SoundNet>("SoundNet");
    assert_schema_is_usable::<CaughtUpNet>("CaughtUpNet");

    assert_schema_is_usable::<LifecyclePhaseNet>("LifecyclePhaseNet");
    assert_schema_is_usable::<RunningPhaseNet>("RunningPhaseNet");
    assert_schema_is_usable::<GamePhaseNet>("GamePhaseNet");
    assert_schema_is_usable::<BattleScapePhaseNet>("BattleScapePhaseNet");
    assert_schema_is_usable::<AfterMathPhaseNet>("AfterMathPhaseNet");
    assert_schema_is_usable::<AppPhaseNet>("AppPhaseNet");

    assert_schema_is_usable::<ActCountNet>("ActCountNet");
    assert_schema_is_usable::<AppPhaseTargetNet>("AppPhaseTargetNet");
    assert_schema_is_usable::<WaitConditionNet>("WaitConditionNet");
}

#[test]
fn the_published_refusal_shape_names_the_suppressed_variant() {
    let Ok(doc) = shape_trace::<CostRefusalNet>() else {
        unreachable!("`CostRefusalNet`'s shape must trace out of its own Deserialize impl")
    };
    let Some(ShapeBody::Choice(variants)) = doc.root_body() else {
        unreachable!("`CostRefusalNet` publishes a choice of variants: {doc:?}")
    };
    assert!(
        variants
            .iter()
            .any(|variant| variant.name().as_str() == "Suppressed"),
        "a client reading the published shape must find the Suppressed refusal: {variants:?}",
    );
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
