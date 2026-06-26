//! The authored per-injury record [`InjuryDef`] and its parsed-but-unread
//! [`PostHeal`] placeholder.

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::{InjuryEffect, InjuryName, InspectText, LogText, PopupText};
use crate::{armor::BodyPart, severity::Severity};

/// The **post-heal disposition** of an injury (GTW-23 Healing — NOT yet built).
///
/// The schema CARRIES this slot for forward compatibility; the GTW-405 vocabulary
/// **parses it and never reads it** (the projector and runtime ignore it). Its only
/// live variant is [`Deferred`](PostHeal::Deferred). GTW-23 will add the reserved
/// shapes — documented here, deliberately NOT yet declared as variants so they
/// carry no dead, untested code:
///
/// - `Clean(..)` — a clean heal that fully clears the injury (often no residual
///   effect).
/// - `Partial(..)` — a partial heal that leaves a permanent residual (usually a
///   lasting stat debuff, possibly with extra penalties).
///
/// A pure value enum (no bare marker). [`Deserialize`] so the `.injury.ron`
/// `post_heal:` field parses; defaults to [`Deferred`](PostHeal::Deferred) via
/// [`deferred`](PostHeal::deferred) so floor files may omit it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize)]
pub enum PostHeal {
    /// Healing semantics are deferred to GTW-23 — the only live variant. Parsed,
    /// stored, and never read at runtime in GTW-405.
    #[default]
    Deferred,
}

impl PostHeal {
    /// The default disposition — [`Deferred`](PostHeal::Deferred). Used as the
    /// `#[serde(default = "PostHeal::deferred")]` source so an authored injury may
    /// omit `post_heal:` entirely.
    #[must_use]
    pub const fn deferred() -> Self {
        Self::Deferred
    }
}

/// One authored **injury definition** — the named condition a non-graze, non-fatal
/// wound can inflict, loaded from `assets/injuries/<part>/<key>.injury.ron`
/// (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// The de-serialization target of the `.injury.ron` schema (the file stem minus
/// `.injury` is the table-build key — an [`InjuryName`] the loader captures, GTW-437).
/// Its fields are ALL typed domain values (no bare string / int / enum): the display
/// [`name`](InjuryDef::name), the struck [`body_part`](InjuryDef::body_part) (the
/// REUSED [`BodyPart`] — the def's own field is authoritative; the owning subfolder
/// is only organizational), the rolled [`severity`](InjuryDef::severity) (the REUSED
/// [`Severity`] — only `Minor` / `Major` / `Critical` are tabled; `None` is a graze
/// and `Fatal` is death via the existing gate), the three routed texts, and the
/// `≥ 1` [`effects`](InjuryDef::effects). The [`post_heal`](InjuryDef::post_heal)
/// slot is parsed but unread (GTW-23).
///
/// Public fields (a value-object record, mirroring
/// [`InflictedWound`](crate::inflicted_wound::InflictedWound)) — every field is a
/// named domain type, so a literal is self-documenting.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct InjuryDef {
    /// The display name of the condition (e.g. `"Lost Eye"`) — the inspect-panel
    /// label.
    pub name:         InjuryName,
    /// The struck body part — authoritative; a subfolder mismatch only WARNs (GTW-437).
    pub body_part:    BodyPart,
    /// The severity bucket this injury belongs to (`Minor` / `Major` / `Critical`).
    pub severity:     Severity,
    /// The FCT popup line shown on infliction.
    pub popup_text:   PopupText,
    /// The combat-log clause shown on infliction.
    pub log_text:     LogText,
    /// The persistent inspect-panel description.
    pub inspect_text: InspectText,
    /// The one-or-more effects this injury applies (`≥ 1`).
    pub effects:      Vec<InjuryEffect>,
    /// The post-heal disposition (parsed but unread in GTW-405). Defaults to
    /// [`PostHeal::Deferred`].
    #[serde(default = "PostHeal::deferred")]
    pub post_heal:    PostHeal,
}
