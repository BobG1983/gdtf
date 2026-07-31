//! The ONE place a JSON Schema document is produced.

use schemars::JsonSchema;

/// The JSON text of `T`'s DERIVED schema.
///
/// `schemars` builds it from the type; nothing here writes schema text, which is what
/// keeps the catalogue's published shape and the decoder's accepted shape the same shape.
///
/// A serializer failure is effectively impossible for a `Schema` (it wraps a JSON value
/// that came from serde in the first place) but must still be handled without panicking,
/// so a permissive empty object is the fallback — visibly wrong in a catalogue rather than
/// a crashed QA channel.
pub(crate) fn schema_text<T: JsonSchema>() -> String {
    let schema = schemars::schema_for!(T);
    serde_json::to_string(&schema).unwrap_or_else(|_| "{\"type\":\"object\"}".to_owned())
}
