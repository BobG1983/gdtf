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
//!
//! E1.4 ([`cover`]) adds the [`cover::CoverLedger`] resource — the single
//! authoritative store of cover structural HP, ONE unified map keyed
//! `(cell, level)` for BOTH walls and props, lazily seeded to `max_hp` on first
//! access. [`cover::CoverLedger::deplete_cover`] spends HP and emits a
//! [`cover::CoverEvent::Destroyed`] marker at zero (the occupancy/prop
//! consequences are deferred to GTW-35). Cover reuses the GTW-153 armor newtypes,
//! and band thresholds come from [`tuning::CombatTuning`] via [`cover::band_for`].
//! See `docs/combat/resolution.md` §3 and `docs/architecture.md`.
//!
//! E1.5 ([`surface`]) adds the [`surface::SurfaceGrid`] resource — the persistent
//! store of floor/roof slab existence ([`surface::SlabState`] per [`metric::CellLevel`])
//! and per-[`metric::Cell`] ground damage ([`surface::GroundDamage`]), mutated **in
//! place** so it survives every occupancy rebuild. A destroyed slab stays destroyed
//! ([`surface::SurfaceGrid::destroy_slab`] is terminal) and ground damage only
//! accrues ([`surface::SurfaceGrid::accrue_ground_damage`] is monotonic). This is a
//! SEPARATE resource from the coarse occupancy (E1.6 / GTW-156). See
//! `docs/architecture.md`'s "persistent surface grid" + the surface/ground-hit verbs.

pub mod armor;
pub mod cover;
pub mod ganger;
pub mod metric;
pub mod surface;
pub mod tuning;

pub use armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, BodyPart, SourceArmor,
    WornArmor,
};
pub use cover::{
    BandHeightPx, CoverDamage, CoverEntry, CoverEvent, CoverHp, CoverLedger, Destroyed, HeightBand,
    band_for,
};
pub use ganger::{
    Aiming, Direction, Facing, Faction, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds,
};
pub use metric::{BattlePx, CELL_PITCH_PX, Cell, CellLevel, Level, MAX_LEVELS, Z_LEVEL_HEIGHT};
pub use surface::{GroundDamage, SlabState, SurfaceGrid};
pub use tuning::{
    BandEdgePx, BodyPartWeight, BodyPartWeights, CombatTuning, DefenderLuckSpreadCap,
    PenDamageScale, ProjectileBandEdges, RandomSpread, RandomSpreadMin, SeverityScaling,
    ShooterLuckScale, ToughnessMitigation,
};
