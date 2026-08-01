//! The call-arguments fixture the arms in this module render with (GTW-942).

use serde_json::{Value, json};

/// The empty `arguments` object every arm in this module is rendered with.
///
/// `render_response` takes the CALL's arguments because the `commands` arm reads its detail
/// level and its one-command filter from them (GTW-942); no arm covered here does, so they
/// all pass the same empty object rather than each inventing one.
pub(super) fn no_args() -> Value {
    json!({})
}
