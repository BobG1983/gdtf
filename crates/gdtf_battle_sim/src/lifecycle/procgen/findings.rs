//! The procgen **content-integrity findings** — the typed record of every
//! degraded resolution the emit step used to take SILENTLY (GTW-582 C3(d)/C5).
//!
//! The emit step is render-free, deterministic MODEL logic with no ECS access,
//! so it cannot write the app's content-integrity report itself. Instead it
//! RETURNS these per-family findings ([`EmittedLevel::findings`]) and the
//! app-side procgen driver converts them into report entries + `warn!`s — the
//! sim stays render-free and pure, the degradation still lands on the record
//! (never silent). Per-family finding types live with their family (gate
//! directive P10): these are the PROCGEN family's.

use crate::{level::ThemeUuid, situation::Situation, terrain::def::TerrainUuid};

/// One degraded resolution the procgen emit took while pouring a level — a
/// LOUD-but-non-fatal record, never an error (the emit stays infallible;
/// connectivity is by-construction, GTW-497).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcgenFinding {
    /// The requested theme resolved NO default floor in the
    /// [`UuidThemeRegistry`](crate::level::UuidThemeRegistry) — the level was
    /// poured with the NIL-sentinel floor (playable but degraded: setup then
    /// skips registry floor resolution and falls back to the tuning move
    /// cost). Pre-GTW-582 this fallback was silent.
    MissingThemeDefaultFloor {
        /// The theme UUID that resolved no registry entry.
        theme: ThemeUuid,
    },
    /// A placed piece's terrain UUID resolved NO
    /// [`TerrainDef`](crate::terrain::def::TerrainDef) — the piece was poured
    /// FAIL-OPEN into the walls list so it surfaces as a
    /// [`TerrainNotFound`](crate::situation::BattleSetupError::TerrainNotFound)
    /// at setup rather than vanishing. Pre-GTW-582 the pour itself was silent.
    UnresolvedTerrainPiece {
        /// The unresolved terrain-definition UUID.
        piece: TerrainUuid,
    },
}

/// The emit step's full result: the poured [`Situation`] plus every degraded
/// resolution taken while pouring it (GTW-582 — a named result struct, not a
/// bare tuple, per the no-bare-types rule).
#[derive(Debug, Clone)]
pub struct EmittedLevel {
    /// The poured level — the sim's canonical battlefield value.
    pub situation: Situation,
    /// Every degraded resolution the pour took, deduplicated and in
    /// deterministic (first-encounter) order; EMPTY on a fully-resolved pour.
    pub findings:  Vec<ProcgenFinding>,
}
