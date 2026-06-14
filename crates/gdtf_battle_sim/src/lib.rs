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
//!
//! E1.3 ([`armor`]) adds the four armor-stat newtypes, an [`armor::ArmorPiece`]
//! per body location, the read-only roster [`armor::SourceArmor`] record, and the
//! battle-local [`armor::WornArmor`] component seeded by value from it — the sim's
//! only mutable armor surface during a battle. See
//! `docs/combat/weapons-and-armor.md` and `docs/architecture.md`.

pub mod armor;
pub mod ganger;
pub mod metric;
pub mod tuning;

pub use armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, BodyPart, SourceArmor,
    WornArmor,
};
pub use ganger::{
    Aiming, Direction, Facing, Faction, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds,
};
pub use metric::{BattlePx, CELL_PITCH_PX, Cell, CellLevel, Level, MAX_LEVELS, Z_LEVEL_HEIGHT};
pub use tuning::{
    BandEdgePx, BodyPartWeight, BodyPartWeights, CombatTuning, DefenderLuckSpreadCap,
    PenDamageScale, ProjectileBandEdges, RandomSpread, RandomSpreadMin, SeverityScaling,
    ShooterLuckScale, ToughnessMitigation,
};
