//! The authored per-injury record [`InjuryDef`] and its parsed-but-unread
//! [`PostHeal`] placeholder.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{InjuryEffect, InjuryName, InspectText, LogText, PopupText};
use crate::{armor::InjuryCategory, severity::Severity};

/// The **post-heal disposition** of an injury (GTW-23 Healing — **NOT yet built**).
///
/// The schema CARRIES this slot for forward compatibility; the GTW-405 vocabulary
/// **parses it and never reads it** (the projector and runtime ignore it). Its only
/// live variant is [`Deferred`](PostHeal::Deferred). GTW-23 will add the reserved
/// shapes — documented here, deliberately NOT yet declared as variants so they
/// carry no dead, untested code.
///
/// **Authoring note (GTW-23):** omitting `post_heal:` from an `.injury.ron` file is
/// the correct and expected pattern — the field defaults to `Deferred` automatically.
/// When GTW-23 Healing lands it will add `Clean` and `Partial` variants here and the
/// runtime will begin reading this field; until then the authored value is schema-only.
///
/// - `Clean(..)` — a clean heal that fully clears the injury (often no residual
///   effect).
/// - `Partial(..)` — a partial heal that leaves a permanent residual (usually a
///   lasting stat debuff, possibly with extra penalties).
///
/// A pure value enum (no bare marker). [`Deserialize`] so the `.injury.ron`
/// `post_heal:` field parses; defaults to [`Deferred`](PostHeal::Deferred) via
/// [`deferred`](PostHeal::deferred) so floor files may omit it. `Serialize` is
/// added (GTW-654) so the content editor's INJURY authoring mode can write an
/// edited [`InjuryDef`] back to disk through the shared RON save seam (the
/// [`ArmorSpec`](crate::armor::ArmorSpec) / `GangRoster` precedent) — behavior-inert
/// for the sim.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
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
/// wound can inflict, loaded from `assets/content/injuries/<part>/<key>.injury.ron`
/// (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// The de-serialization target of the `.injury.ron` schema (the file stem minus
/// `.injury` is the table-build key — an [`InjuryName`] the loader captures, GTW-437).
/// Its fields are ALL typed domain values (no bare string / int / enum): the display
/// [`name`](InjuryDef::name), the injury-pool [`category`](InjuryDef::category) (the
/// shared [`InjuryCategory`] this def is authored into — the table-key axis; the owning
/// subfolder must match it but is only organizational), the rolled
/// [`severity`](InjuryDef::severity) (the REUSED
/// [`Severity`] — only `Minor` / `Major` / `Critical` are tabled; `None` is a graze
/// and `Fatal` is death via the existing gate), the three routed texts, and the
/// `≥ 1` [`effects`](InjuryDef::effects). The [`post_heal`](InjuryDef::post_heal)
/// slot is parsed but unread (GTW-23).
///
/// Public fields (a value-object record, mirroring
/// [`InflictedWound`](crate::inflicted_wound::InflictedWound)) — every field is a
/// named domain type, so a literal is self-documenting.
///
/// Derives [`PartialEq`] but NOT [`Eq`] (GTW-444): its `effects` may carry a
/// [`MovementCostMul`](super::InjuryEffect::MovementCostMul) whose `f32` payload is not
/// `Eq`. The registry keys defs by [`InjuryName`] (a `String` newtype), never by the
/// whole def, so no `Eq`/`Hash` on `InjuryDef` is needed.
///
/// Derives [`Serialize`] too (GTW-654): the content editor's INJURY authoring mode
/// WRITES an edited def back to a `.injury.ron` through the shared RON save seam
/// (the [`ArmorSpec`](crate::armor::ArmorSpec) / `GangRoster` write precedent), so the
/// authoring struct must serialise to exactly the shape it deserialises from.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct InjuryDef {
    /// The display name of the condition (e.g. `"Lost Eye"`) — the inspect-panel
    /// label.
    pub name:         InjuryName,
    /// The injury-pool [`InjuryCategory`] this def routes into — the table-KEY axis
    /// (GTW-453). A cross-category subfolder mismatch only WARNs (GTW-437).
    pub category:     InjuryCategory,
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
    /// The post-heal disposition (parsed but **unread** in GTW-405 — GTW-23 Healing owns
    /// the semantics). Defaults to [`PostHeal::Deferred`]; omitting it from an authored
    /// `.injury.ron` is the correct pattern — the field is schema-forward-compat only.
    #[serde(default = "PostHeal::deferred")]
    pub post_heal:    PostHeal,
}
