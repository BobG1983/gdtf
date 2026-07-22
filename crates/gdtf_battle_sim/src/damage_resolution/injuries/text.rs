//! The injury **string newtypes** — the [`InjuryName`] identity and the three
//! routed display texts ([`PopupText`] / [`LogText`] / [`InspectText`]).

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

// Every newtype below also derives `Serialize` (GTW-654): the content editor's
// INJURY authoring mode writes an edited `InjuryDef` / `InjuryWeighting` back to
// disk through the shared RON save path (the `ArmorSpec` / `GangRoster`
// precedent), and these are their string leaves — behavior-inert for the sim.

/// An injury's **name** — serving BOTH roles the schema needs: the human display
/// label (the `name` field of an [`InjuryDef`](super::InjuryDef), e.g. `"Lost Eye"`,
/// the named condition shown in the inspect panel) AND the table-build KEY (the
/// `.injury.ron` file stem, e.g. `"lost_eye"`, that a
/// [`WeightedInjuryEntry`](super::WeightedInjuryEntry) references and the loader
/// resolves to its [`InjuryDef`](super::InjuryDef)).
///
/// They are the SAME type used in two roles: the distinction lives in the DATA
/// (which string), not in the type — so the loader can key a registry by the
/// stem-`InjuryName` and the panel can read the display-`InjuryName` without a
/// second vocabulary. A no-bare-types newtype over [`String`] (a name is a domain
/// value): private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
/// RON string. [`Eq`] / [`Hash`] so it can key the GTW-437 injury registry, and
/// [`Ord`] so the loader can canonically sort a bucket's weighting rows by key (making
/// the cumulative-weight pick folder-enumeration-order-independent).
#[derive(Deref, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InjuryName(String);

impl InjuryName {
    /// Build an injury name from its string (a display label OR a file-stem key).
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// An injury's **popup text** — the short, upper-case line routed to the floating
/// combat text (FCT) on infliction (e.g. `"LOST EYE"`).
///
/// One of three distinct display destinations (no-bare-types rule 3: distinct
/// concepts get distinct newtypes): this one feeds the presenter's FCT pop, valence
/// coloured by severity. A newtype over [`String`]: private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON string. The presenter
/// routing is GTW-439; THIS slice only carries the text.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PopupText(String);

impl PopupText {
    /// Build an injury popup text from its FCT string.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}

/// An injury's **log text** — the combat-log clause routed to the rolling log on
/// infliction (e.g. `"loses an eye"`, rendered as `"<name> loses an eye"`).
///
/// The second of three distinct display destinations: this one feeds the combat-log
/// line classifier. A newtype over [`String`]: private inner + derived [`Deref`];
/// `#[serde(transparent)]` parses a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LogText(String);

impl LogText {
    /// Build an injury log text from its combat-log clause string.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}

/// An injury's **inspect text** — the persistent per-ganger injury-list line shown
/// in the inspect panel (e.g. `"Lost Eye -- -2 Aim, -1 Cool"`).
///
/// The third of three distinct display destinations: this one is the durable
/// description carried on the [`GainedInjury`](super::GainedInjury) ledger entry
/// (the message drives the transient FCT/log flash, the ledger drives this
/// persistent list). A newtype over [`String`]: private inner + derived [`Deref`];
/// `#[serde(transparent)]` parses a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InspectText(String);

impl InspectText {
    /// Build an injury inspect text from its panel-description string.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}
