//! [`ShotName`] — the capture file stem a caller chooses (GTW-734, re-homed by GTW-943).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// A requested capture's **file stem** — the caller-chosen name a
/// [`CaptureRider`](crate::command::CaptureRider) writes under.
///
/// The host constrains the actual path under its own capture directory (GTW-694); this is
/// only the stem the client asks for. A name newtype over `String` (no-bare-types),
/// serde-transparent. `Clone`-not-`Copy` (holds a `String`).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShotName(String);

impl ShotName {
    /// Build a capture name from its file stem.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
