//! The **damage context** — the source/kind of the wound that a non-graze, non-fatal
//! injury roll samples its weighting table under (GTW-452).
//!
//! The GTW-440 injury pool is ONE shared per-category catalog; GTW-452 makes injury
//! SELECTION source-specific by adding a per-context weighting dimension over that SAME
//! pool. A wound carries the context of the blow that caused it — a [`Ranged`](DamageContext::Ranged)
//! shot, a [`Melee`](DamageContext::Melee) strike, or a [`Fall`](DamageContext::Fall) — and the
//! roll keys its [`InjuryTables`](super::InjuryTables) bucket by
//! `(category, context, severity)`, so the SAME shared injury (e.g. a `broken_nose`) can be
//! weighted meaningfully in one context and near-zero in another WITHOUT duplicating the
//! injury definition itself (`docs/combat/resolution.md` injury tables).

use serde::{Deserialize, Serialize};

/// The **source of a wound** — which damage path caused the hit the injury roll is
/// weighting for (GTW-452).
///
/// The per-context axis of the injury weighting tables: a wound rolls its named injury
/// from the `(category, context, severity)` bucket, so the ONE shared per-category injury
/// pool (GTW-440) can be weighted DIFFERENTLY per source — a face-first melee blow weights
/// a `broken_nose` high, a ranged shot near-zero, a fall weights a `twisted_ankle` — with
/// no duplicated injury definition (the def is the same; only the weight differs). The
/// three variants match exactly what the landed damage paths distinguish today: the ranged
/// fire fold (GTW-198), the melee strike (GTW-37), and the fall (GTW-39).
///
/// **Which contexts a production path constructs today:** [`Ranged`](DamageContext::Ranged)
/// on the fire fold and [`Fall`](DamageContext::Fall) on the fall path.
/// [`Melee`](DamageContext::Melee) is constructed by NO production path yet — the melee
/// strike ([`resolve_melee_strike`](crate::melee::resolve_melee_strike)) stops at the §6
/// wound and never runs the §8 injury roll — so the shipped
/// `weighting/<category>.melee.weighting.ron` tables load and build but are DORMANT until
/// the melee §8 wiring lands (`docs/combat/resolution.md` §7; the dormant-table note in
/// `docs/authoring/injury-authoring.md`).
///
/// A pure value enum (no bare integer / string for the source axis). `Hash`/`Eq` so it keys
/// the [`InjuryTables`](super::InjuryTables) map alongside the category + severity.
/// [`Deserialize`] so a `.weighting.ron` file names its context by the `context:` field;
/// [`Serialize`] so the content editor's INJURY authoring mode round-trips it. [`Default`]
/// is [`Ranged`](DamageContext::Ranged) — the ordinary combat source — so a weighting file
/// authored before GTW-452 (no `context:` field) parses as a ranged table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum DamageContext {
    /// A **ranged** hit — a shot from the fire pipeline (GTW-198). The default source.
    #[default]
    Ranged,
    /// A **melee** hit — a connecting strike from the §7 opposed-Fight (GTW-37). Its
    /// weighting tables are authored + built but DORMANT: the melee strike path runs no §8
    /// injury roll yet, so nothing constructs this variant outside tests (see the type docs).
    Melee,
    /// A **fall** — an unsupported drop's kinetic landing damage (GTW-39).
    Fall,
}

impl DamageContext {
    /// The three damage contexts in canonical order — the iteration order the
    /// missing-weighting audit walks (an injury is rollable if ANY context weights it) and
    /// the exhaustive sweep the tests cover.
    pub const ALL: [Self; 3] = [Self::Ranged, Self::Melee, Self::Fall];
}
