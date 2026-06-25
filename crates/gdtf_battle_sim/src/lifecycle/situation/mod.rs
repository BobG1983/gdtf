//! The canonical authored **situation** and the setup that pours it into the
//! battle ECS — the E1.8 situation→entities slice. The setup systems here build the
//! scene from the situation (floor, walls, scatter, gangers); the situation is
//! gangers, walls, scatter, upper-floor slabs, and stair/ladder vertical links —
//! every placement an optional storey, position always the pair (cell, level). This
//! module is the in-crate **setup-on-entry source of truth** for that construction.
//!
//! [`Situation`] is the **one canonical** authored battlefield value — it
//! supersedes the GTW-156 placeholder (renamed to
//! [`crate::occupancy::OccupancyInput`], the grid-relevant construction input) and
//! carries every authored field:
//!
//! - **gangers** ([`GangerSpawn`]): each a `(cell, level)` plus the E1.2 component
//!   VALUES ([`Faction`](crate::ganger::Faction) / [`Facing`](crate::ganger::Facing)
//!   / [`Stance`](crate::ganger::Stance) / [`Aiming`](crate::ganger::Aiming) /
//!   [`Hp`](crate::ganger::Hp) / [`Wounds`](crate::ganger::Wounds) /
//!   [`Tu`](crate::ganger::Tu) / [`LifeState`](crate::ganger::LifeState)), the E3.0
//!   attribute stats ([`Shooting`](crate::ganger::Shooting) /
//!   [`Toughness`](crate::ganger::Toughness) / [`Luck`](crate::ganger::Luck)), a
//!   read-only [`SourceArmor`](crate::armor::SourceArmor) record (`armor_by_part`) and a
//!   [`weapon`](GangerSpawn::weapon) KEY resolved against the
//!   [`WeaponRegistry`](crate::weapon::WeaponRegistry) into the spawned
//!   [`WeaponBundle`](crate::weapon::WeaponBundle) (GTW-257).
//! - **walls** + **scatter/props** ([`CoverSpawn`], the same schema for both): each
//!   a `(cell, level)`, a [`TerrainKind`](crate::occupancy::TerrainKind), the cover's
//!   max [`CoverHp`](crate::cover::CoverHp), its [`HeightBand`](crate::cover::HeightBand),
//!   and its armor stats ([`ArmorProtection`](crate::armor::ArmorProtection) /
//!   [`ArmorHardness`](crate::armor::ArmorHardness)).
//! - **slabs** (a `Vec<`[`CellLevel`](crate::metric::CellLevel)`>`): the `(cell, level)`s
//!   that carry a present floor / roof slab.
//! - **`vertical_links`** (a `Vec<`[`VerticalLink`](crate::vertical::VerticalLink)`>`,
//!   moved here from the GTW-156 placeholder): the authored stair / ladder links (E1.10).
//!
//! [`setup_battle`] reads a [`Situation`] and builds the battle in the ECS world
//! (the setup systems described above). It is **render-free** and driven from
//! a headless `MinimalPlugins` app (it takes only [`Commands`](bevy::prelude::Commands)
//! — no renderer, no asset server). For each ganger it `commands.spawn_scene(...)`s its
//! own per-field components (no equipment stat data — GTW-323, ADR-0004) and relates its
//! weapon entity ([`Wields`](crate::weapon::Wields)) + six armor-piece entities
//! ([`Wears`](crate::armor::Wears)), capturing the returned Bevy
//! [`Entity`](bevy::prelude::Entity) handle — **never a numeric id** (GTW-10 / GTW-12).
//! It then seeds the
//! [`CoverLedger`](crate::cover::CoverLedger) (E1.4) from the walls + scatter, the
//! [`SurfaceGrid`](crate::surface::SurfaceGrid) (E1.5) from the slabs, and the
//! [`OccupancyGrid`](crate::occupancy::OccupancyGrid) (E1.6) from the authored terrain
//! plus the SPAWNED occupant entities
//! (via [`OccupancyGrid::build_from_occupancy_input`](crate::occupancy::OccupancyGrid::build_from_occupancy_input)),
//! inserting each as a resource. Finally it validates and inserts the
//! [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) (E1.10), returning
//! [`InvalidVerticalLink`](crate::vertical::InvalidVerticalLink) if an authored
//! link is bad (the no-panic contract).

mod error;
mod setup;
mod spawn;
mod terrain_resolve;

#[cfg(test)]
mod test;

pub use error::BattleSetupError;
pub use setup::{BattleSetup, has_stacked_gangers, setup_battle};
pub use spawn::{CoverSpawn, FloorSpawn, GangerSpawn, Situation, SlabSpawn};
