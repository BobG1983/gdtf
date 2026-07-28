//! The `query_editor` `topic` argument ⇄ [`EditorQueryKind`] mapping (GTW-808).
//!
//! One place owns the topic NAMES a client types: the `query_editor` input schema
//! publishes them as its `enum`, and the argument parser resolves them back. Both walk
//! [`EditorQueryKind::ALL`], so a topic added to the protocol appears in the schema and
//! becomes callable without a second edit here.

use gdtf_qa_protocol::view::EditorQueryKind;

/// The wire name a client passes for `topic`.
///
/// A wildcard-free `match`, so a new [`EditorQueryKind`] must be named rather than
/// silently reading as its neighbour. The names are the protocol enum's own variant
/// spellings, which is what makes a reply's `view` tag and the request's `topic` read the
/// same in a transcript.
#[must_use]
pub(super) const fn topic_wire_name(kind: EditorQueryKind) -> &'static str {
    match kind {
        EditorQueryKind::Readiness => "Readiness",
        EditorQueryKind::Mode => "Mode",
        EditorQueryKind::Session => "Session",
        EditorQueryKind::Draft => "Draft",
        EditorQueryKind::Validation => "Validation",
    }
}

/// Every legal `topic` value, in topic-declaration order — the schema's `enum` list and
/// the rejection message's "one of" list.
#[must_use]
pub(super) fn topic_wire_names() -> Vec<&'static str> {
    EditorQueryKind::ALL
        .iter()
        .copied()
        .map(topic_wire_name)
        .collect()
}

/// Resolve a `topic` argument to its [`EditorQueryKind`], or `None` for a name no topic
/// claims.
#[must_use]
pub(super) fn topic_from_wire(name: &str) -> Option<EditorQueryKind> {
    EditorQueryKind::ALL
        .iter()
        .copied()
        .find(|kind| topic_wire_name(*kind) == name)
}

#[cfg(test)]
mod test {
    use gdtf_qa_protocol::view::EditorQueryKind;

    use super::{topic_from_wire, topic_wire_name, topic_wire_names};

    /// Every protocol topic has a name, and every name resolves back to its topic — so the
    /// schema can never advertise a topic the parser rejects.
    #[test]
    fn every_topic_round_trips_through_its_wire_name() {
        for kind in EditorQueryKind::ALL {
            assert_eq!(topic_from_wire(topic_wire_name(kind)), Some(kind));
        }
        assert_eq!(topic_wire_names().len(), EditorQueryKind::ALL.len());
    }

    /// The names are the protocol enum's own spellings, so a request's `topic` and a
    /// reply's `view` tag read identically.
    #[test]
    fn the_names_match_the_protocol_serialization() {
        for kind in EditorQueryKind::ALL {
            let Ok(serialized) = serde_json::to_value(kind) else {
                unreachable!("an editor query kind serializes");
            };
            assert_eq!(serialized, serde_json::json!(topic_wire_name(kind)));
        }
    }

    /// An unknown topic name resolves to nothing rather than to a default topic.
    #[test]
    fn an_unknown_topic_name_resolves_to_nothing() {
        assert_eq!(topic_from_wire("readiness"), None);
        assert_eq!(topic_from_wire("Everything"), None);
    }
}
