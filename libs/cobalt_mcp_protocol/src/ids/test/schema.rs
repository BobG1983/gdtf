use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::{
    command::{
        ArgSchemaRon, CommandAvailability, CommandEntry, CommandName, CommandSummary,
        CommandTiming, ReplySchemaRon, RonShape, ShapeBody, ShapeDoc, shape_text,
    },
    ids::ShotName,
    test_support::assert_ron_round_trip,
};

/// First axis of a probe pair.
#[derive(Deref, Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ProbeFirst(i32);

/// Second axis of a probe pair.
#[derive(Deref, Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ProbeSecond(i32);

#[derive(Debug, Serialize, Deserialize)]
struct ProbePair {
    first:  ProbeFirst,
    second: ProbeSecond,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProbeArgs {
    actor: ProbeFirst,
    at:    ProbePair,
    name:  Option<ShotName>,
}

fn parsed_shape(text: &str) -> ShapeDoc {
    let Ok(doc) = ron::de::from_str::<ShapeDoc>(text) else {
        unreachable!("a traced shape is parseable RON, got `{text}`");
    };
    doc
}

#[test]
fn a_traced_shape_over_id_newtypes_names_every_field() {
    let text = shape_text::<ProbeArgs>();
    let doc = parsed_shape(&text);

    let Some(ShapeBody::Record(fields)) = doc.root_body() else {
        unreachable!("the root of a named-field struct is a record: `{text}`");
    };
    let named: Vec<&str> = fields.iter().map(|field| field.name().as_str()).collect();
    assert_eq!(
        named,
        vec!["actor", "at", "name"],
        "the traced shape names every field in order: `{text}`",
    );
    let Some(name_field) = fields.iter().find(|field| field.name().as_str() == "name") else {
        unreachable!("the `name` field was just found above");
    };
    assert!(
        matches!(name_field.shape(), RonShape::Optional(_)),
        "an `Option` field is Optional in the shape: `{text}`",
    );
}

#[test]
fn a_traced_shape_survives_the_ron_round_trip_inside_a_catalogue_row() {
    let arguments = ArgSchemaRon::new(shape_text::<ProbeArgs>());
    let reply = ReplySchemaRon::new(shape_text::<ProbePair>());
    let entry = CommandEntry::new(
        CommandName::from_static("probe.trace"),
        CommandSummary::from_static("A stand-in command whose shapes are really traced."),
        CommandTiming::Immediate,
        arguments.clone(),
        reply.clone(),
        CommandAvailability::Available,
    );

    assert_ron_round_trip(&entry);

    let Ok(encoded) = ron::ser::to_string(&entry) else {
        unreachable!("a catalogue row serializes to compact RON");
    };
    let Ok(decoded) = ron::de::from_str::<CommandEntry>(&encoded) else {
        unreachable!("the compact RON `{encoded}` parses back to a CommandEntry");
    };
    assert_eq!(
        decoded.arguments.as_str(),
        arguments.as_str(),
        "the traced argument shape text is unchanged by the round trip",
    );
    assert_eq!(
        decoded.reply.as_str(),
        reply.as_str(),
        "the traced reply shape text is unchanged by the round trip",
    );
}
