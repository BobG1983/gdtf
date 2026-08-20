//! What one faction may be told about a logged act.
//! The one classifier the act-log read and the combat log panel both call.

use bevy::prelude::Deref;

/// Whether a faction could observe an act at all.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActObserved(bool);

impl ActObserved {
    /// Wrap an observed flag.
    #[must_use]
    pub const fn new(observed: bool) -> Self {
        Self(observed)
    }
}

/// Whether a faction could put a name to the entity an act names.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorIdentified(bool);

impl ActorIdentified {
    /// Wrap an identified flag.
    #[must_use]
    pub const fn new(identified: bool) -> Self {
        Self(identified)
    }
}

/// What a faction may be told about one act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActVisibility {
    /// Reported, naming who acted.
    Named,
    /// Reported, naming nobody.
    Unnamed,
    /// Not reported at all.
    Withheld,
}

/// Decide what a faction learns from what it observed and whom it could identify.
/// An act it did not observe is withheld, not blanked.
#[must_use]
pub const fn classify_act(observed: ActObserved, identified: ActorIdentified) -> ActVisibility {
    if !observed.0 {
        ActVisibility::Withheld
    } else if identified.0 {
        ActVisibility::Named
    } else {
        ActVisibility::Unnamed
    }
}
