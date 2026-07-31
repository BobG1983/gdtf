//! The `schema` feature really derives usable JSON Schema documents over the id newtypes
//! (GTW-939).
//!
//! The reason the feature exists: a host declares a command's `Args` / `Reply` as ordinary
//! Rust types built from this crate's wire vocabulary, and the catalogue publishes the
//! DERIVED schema of each. That only works if every id type carries a `JsonSchema` impl,
//! which no downstream crate could add for it (both the trait and the types are foreign
//! there — the orphan rule). These tests pin the two facts a host depends on: the derive
//! compiles over the id types, and what it produces is parseable JSON.

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

use crate::{
    command::{
        ArgSchemaJson, CommandAvailability, CommandEntry, CommandName, CommandSummary,
        ReplySchemaJson,
    },
    ids::{CellLevelNet, GangerToken, ShotName},
    test_support::assert_ron_round_trip,
};

/// A stand-in for a real command's argument type: an ordinary struct built out of this
/// crate's id newtypes, deriving `JsonSchema` exactly as a host's `Args` type does.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ProbeArgs {
    /// The ganger the probe act belongs to.
    actor: GangerToken,
    /// The cell and storey it targets.
    at:    CellLevelNet,
    /// An optional capture stem, so the derive is exercised over an `Option` too.
    name:  Option<ShotName>,
}

/// The derived schema of `T`, as the JSON text a catalogue row would publish.
///
/// `schemars::Schema` is a serde value, not a `Display` type, so the text a host puts on
/// the wire is what `serde_json` writes for it.
fn schema_text<T: JsonSchema>() -> String {
    let Ok(text) = serde_json::to_string(&schema_for!(T)) else {
        unreachable!("a derived schema serializes to JSON text");
    };
    text
}

/// The derived schema of a struct built from id newtypes is parseable JSON, and describes
/// the fields it was derived from.
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

/// A catalogue row carrying REAL derived-schema text survives the compact-RON round trip
/// unchanged — the JSON-inside-RON encoding the design chose, proven against a schema this
/// test derived rather than a literal copied from one.
#[test]
fn a_derived_schema_survives_the_ron_round_trip_inside_a_catalogue_row() {
    let arguments = ArgSchemaJson::new(schema_text::<ProbeArgs>());
    let reply = ReplySchemaJson::new(schema_text::<CellLevelNet>());
    let entry = CommandEntry::new(
        CommandName::from_static("probe.act"),
        CommandSummary::from_static("A stand-in command whose schemas are really derived."),
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
