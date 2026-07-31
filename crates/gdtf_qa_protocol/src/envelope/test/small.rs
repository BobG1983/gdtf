//! Exhaustive round-trip + parity-forcing pins for the small envelope enums — the
//! inject receipt, reject reason, error, and screenshot result (GTW-734).

use crate::{
    envelope::{
        AutoRunNet, CaptureAimNet, HelloFacts, InjectReceipt, ProtocolVersion, QaError,
        RejectReason, ScreenshotAfterResult, ScreenshotPathNet, ScreenshotResult, ServerNameNet,
        StepperCommandNet, StepperReceipt,
    },
    test_support::assert_ron_round_trip,
};

/// Every [`RejectReason`] round-trips; the witness forces new variants in.
#[test]
fn reject_reason_round_trips_every_variant() {
    for reason in [
        RejectReason::NoBattle,
        RejectReason::NotOffered,
        RejectReason::UnknownEntity,
        RejectReason::BadFireMode,
        RejectReason::StaleToken,
        RejectReason::Unavailable,
    ] {
        match reason {
            RejectReason::NoBattle
            | RejectReason::NotOffered
            | RejectReason::UnknownEntity
            | RejectReason::BadFireMode
            | RejectReason::StaleToken
            | RejectReason::Unavailable => {}
        }
        assert_ron_round_trip(&reason);
    }
}

/// Both [`InjectReceipt`] variants round-trip; the witness forces new variants in.
#[test]
fn inject_receipt_round_trips_every_variant() {
    for receipt in [
        InjectReceipt::Queued,
        InjectReceipt::Rejected(RejectReason::NotOffered),
    ] {
        match receipt {
            InjectReceipt::Queued | InjectReceipt::Rejected(_) => {}
        }
        assert_ron_round_trip(&receipt);
    }
}

/// Every [`QaError`] round-trips; the witness forces new variants in.
#[test]
fn qa_error_round_trips_every_variant() {
    for error in [
        QaError::Busy,
        QaError::VersionMismatch,
        QaError::NoBattle,
        QaError::BadRequest,
        QaError::NotCaughtUp,
        QaError::Timeout,
        QaError::StepperInactive,
        QaError::Malformed,
        QaError::NotNegotiated,
    ] {
        match error {
            QaError::Busy
            | QaError::VersionMismatch
            | QaError::NoBattle
            | QaError::BadRequest
            | QaError::NotCaughtUp
            | QaError::Timeout
            | QaError::StepperInactive
            | QaError::Malformed
            | QaError::NotNegotiated => {}
        }
        assert_ron_round_trip(&error);
    }
}

/// Every [`StepperCommandNet`] form round-trips (the `Next` / `Skip` units and the `Auto`
/// on/off carrier), as does each [`StepperReceipt`]; the witnesses force new variants in.
#[test]
fn stepper_command_and_receipt_round_trip_every_variant() {
    for command in [
        StepperCommandNet::Next,
        StepperCommandNet::Skip,
        StepperCommandNet::Auto {
            running: AutoRunNet::new(true),
        },
        StepperCommandNet::Auto {
            running: AutoRunNet::new(false),
        },
    ] {
        match command {
            StepperCommandNet::Next | StepperCommandNet::Skip | StepperCommandNet::Auto { .. } => {}
        }
        assert_ron_round_trip(&command);
    }
    for receipt in [StepperReceipt::Latched, StepperReceipt::Inactive] {
        match receipt {
            StepperReceipt::Latched | StepperReceipt::Inactive => {}
        }
        assert_ron_round_trip(&receipt);
    }
}

/// Every [`ScreenshotResult`] variant + the [`HelloFacts`] handshake round-trip; the
/// witness forces new screenshot variants in.
#[test]
fn screenshot_result_and_hello_facts_round_trip() {
    for result in [
        ScreenshotResult::Saved(ScreenshotPathNet::new(
            "target/qa_screenshots/x.png".to_owned(),
        )),
        ScreenshotResult::TimedOut,
        ScreenshotResult::TargetNotRendered(CaptureAimNet::new(
            "the capture would read Image(..) but the UI camera renders into Window(..)".to_owned(),
        )),
    ] {
        match result {
            ScreenshotResult::Saved(_)
            | ScreenshotResult::TimedOut
            | ScreenshotResult::TargetNotRendered(_) => {}
        }
        assert_ron_round_trip(&result);
    }
    assert_ron_round_trip(&HelloFacts::new(
        ProtocolVersion::new(3),
        ServerNameNet::new("gdtf-dev".to_owned()),
    ));
}

/// Every [`ScreenshotAfterResult`] variant round-trips; the witness forces new
/// variants in.
#[test]
fn screenshot_after_result_round_trips_every_variant() {
    for result in [
        ScreenshotAfterResult::Rejected(RejectReason::NoBattle),
        ScreenshotAfterResult::Saved(ScreenshotPathNet::new(
            "target/qa_screenshots/y.png".to_owned(),
        )),
        ScreenshotAfterResult::TimedOut,
    ] {
        match result {
            ScreenshotAfterResult::Rejected(_)
            | ScreenshotAfterResult::Saved(_)
            | ScreenshotAfterResult::TimedOut => {}
        }
        assert_ron_round_trip(&result);
    }
}
