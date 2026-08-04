use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

use crate::{
    command::{
        ArgSchemaJson, CommandAvailability, CommandEntry, CommandName, CommandSummary,
        CommandTiming, ReplySchemaJson,
    },
    ids::{CellLevelNet, CellNet, ShotName},
    test_support::assert_ron_round_trip,
};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ProbeArgs {
    actor: CellNet,
    at:    CellLevelNet,
    name:  Option<ShotName>,
}

fn schema_text<T: JsonSchema>() -> String {
    let Ok(text) = serde_json::to_string(&schema_for!(T)) else {
        unreachable!("a derived schema serializes to JSON text");
    };
    text
}

#[test]
fn a_derived_schema_over_id_newtypes_is_parseable_json() {
    let text = schema_text::<ProbeArgs>();

    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) else {
        unreachable!("the derived schema is parseable JSON, got `{text}`");
    };
    let Some(object) = parsed.as_object() else {
        unreachable!("a derived schema is a JSON object, got `{text}`");
    };
    assert_eq!(
        object.get("type").and_then(serde_json::Value::as_str),
        Some("object"),
        "the derived schema describes an object: `{text}`",
    );
    let Some(properties) = object
        .get("properties")
        .and_then(serde_json::Value::as_object)
    else {
        unreachable!("the derived schema lists its properties, got `{text}`");
    };
    for field in ["actor", "at", "name"] {
        assert!(
            properties.contains_key(field),
            "the derived schema names the `{field}` field: `{text}`",
        );
    }
}

#[test]
fn a_derived_schema_survives_the_ron_round_trip_inside_a_catalogue_row() {
    let arguments = ArgSchemaJson::new(schema_text::<ProbeArgs>());
    let reply = ReplySchemaJson::new(schema_text::<CellLevelNet>());
    let entry = CommandEntry::new(
        CommandName::from_static("probe.act"),
        CommandSummary::from_static("A stand-in command whose schemas are really derived."),
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
        "the derived argument schema text is unchanged by the round trip",
    );
    assert_eq!(
        decoded.reply.as_str(),
        reply.as_str(),
        "the derived reply schema text is unchanged by the round trip",
    );
}
