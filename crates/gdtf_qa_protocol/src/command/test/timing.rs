//! Round-trip pins for [`CommandTiming`] and the `#[serde(default)]` it backs (GTW-942).
//!
//! A separate file from the sibling `round_trip` suite because it is a separate
//! change-reason: that one pins the GTW-939 vocabulary, this one pins the field GTW-942
//! added beside it and the backward-compatible decode that field's default promises.

use crate::{
    command::{CommandEntry, CommandTiming},
    test_support::assert_ron_round_trip,
};

/// Every [`CommandTiming`] survives the wire, and the default is
/// [`Immediate`](CommandTiming::Immediate).
///
/// The default is load-bearing rather than cosmetic: [`CommandEntry::timing`] is
/// `#[serde(default)]`, so a row encoded before the field existed decodes as whatever
/// `Default` says — and every command written before it existed answered on its claiming
/// frame.
#[test]
fn every_command_timing_round_trips() {
    assert_eq!(
        CommandTiming::default(),
        CommandTiming::Immediate,
        "a row with no timing on the wire must decode as Immediate",
    );
    for case in CommandTiming::ALL {
        // Wildcard-free, so a new timing must be added to `ALL` before it compiles.
        match case {
            CommandTiming::Immediate | CommandTiming::Deferred => {}
        }
        assert_ron_round_trip(&case);
    }
}

/// A catalogue row encoded WITHOUT a `timing` field still decodes, as `Immediate`.
///
/// The compatibility claim the `#[serde(default)]` on that field makes, tested rather than
/// asserted in prose: this is the exact text a pre-GTW-942 host put on the wire.
#[test]
fn a_row_encoded_without_a_timing_decodes_as_immediate() {
    let legacy = r#"(command:"app.phase",summary:"Read it.",arguments:"{}",reply:"{}",availability:Available)"#;
    let Ok(decoded) = ron::de::from_str::<CommandEntry>(legacy) else {
        unreachable!("a row without `timing` must still decode: {legacy}");
    };
    assert_eq!(decoded.timing, CommandTiming::Immediate);
}
