//! Authoritative, render-free combat simulation for GDTF's turn-based battle
//! system — the MODEL.
//!
//! This crate owns combat truth: it is deterministic and unit-testable with an
//! injected seeded RNG, and it depends on Bevy only for ECS plumbing
//! (MinimalPlugins-compatible) — never on a renderer, window, or asset-server.
//! The presenter (`gdtf_battle_presenter`) mirrors this state; combat rules
//! never live in the view.
//!
//! E1.1 lays the foundation: the [`metric`] battle-space px coordinate system
//! (newtypes + the three named constants) and the [`tuning`] combat-tuning
//! resource. See `docs/combat/battle-space.md` and `docs/combat/resolution.md`.
//!
//! E1.2 ([`ganger`]) decomposes ganger battle state into nine **separate**
//! per-field ECS components so a system can query any subset independently — see
//! `docs/combat/combat.md`, `wounds-and-roster.md`, and `stats.md`.

pub mod ganger;
pub mod metric;
pub mod tuning;

pub use ganger::{
    Aiming, Direction, Facing, Faction, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds,
};
pub use metric::{BattlePx, CELL_PITCH_PX, Cell, CellLevel, Level, MAX_LEVELS, Z_LEVEL_HEIGHT};
pub use tuning::{
    BandEdgePx, BodyPartWeight, BodyPartWeights, CombatTuning, DefenderLuckSpreadCap,
    PenDamageScale, ProjectileBandEdges, RandomSpread, RandomSpreadMin, SeverityScaling,
    ShooterLuckScale, ToughnessMitigation,
};
