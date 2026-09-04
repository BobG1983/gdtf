//! Round-trip pins for [`CommandTiming`] and the `#[serde(default)]` it backs.
use crate::{
    command::{CommandEntry, CommandTiming},
    test_support::assert_ron_round_trip,
};

/// `#[serde(default)]`, so a row encoded before the field existed decodes as whatever
#[test]
fn every_command_timing_round_trips() {
    assert_eq!(
        CommandTiming::default(),
        CommandTiming::Immediate,
        "a row with no timing on the wire must decode as Immediate",
    );
    for case in CommandTiming::ALL {
        match case {
            CommandTiming::Immediate | CommandTiming::Deferred => {}
        }
        assert_ron_round_trip(&case);
    }
}

/// The compatibility claim the `#[serde(default)]` on that field makes, tested rather than
#[test]
fn a_row_encoded_without_a_timing_decodes_as_immediate() {
    let legacy = r#"(command:"app.phase",summary:"Read it.",arguments:"{}",reply:"{}",availability:Available)"#;
    let Ok(decoded) = ron::de::from_str::<CommandEntry>(legacy) else {
        unreachable!("a row without `timing` must still decode: {legacy}");
    };
    assert_eq!(decoded.timing, CommandTiming::Immediate);
}
