//! The **first-half assembler** — anchor selection + strict-opposite enemy placement
//! (GTW-424).
//!
//! This is the GTW-424 slice of the staged assembler (424 placement -> 427 fill -> 431
//! emit/trigger): it picks a player anchor from [`ProcgenRng`](crate::rng::ProcgenRng)
//! (C1), places a `>= 10x10` player-spawn prefab there (C1/OQ-5), places an enemy-spawn
//! prefab at the STRICT geometric opposite (C2/OQ-2), and reserves the 1-cell seam around
//! both (OQ-3). Connectivity is by-construction via that seam lattice — GTW-497 removed the
//! old OQ-4 fail-closed connectivity flood / rejection (there is nothing to assert or
//! repair: the seam guarantees every open cell is reachable). It returns the two
//! [`PlacedPrefab`]s; the GTW-427 fill pass and the GTW-431 emit-to-`Situation` step build
//! on top. NOTHING here wires `BattleScapeState` (a later ticket).
//!
//! GTW-732 split the assemble stage into two single-placement steps ([`place_player`] then
//! [`place_enemy`]) so the unified step primitive can drive them one at a time; the ruled
//! composition [`assemble_placement`] / [`assemble_placement_with`] is a thin caller of both.
//!
//! GTW-492 (child T07b of the GTW-476 data-model refactor): the assembler reads the
//! UUID-keyed [`PrefabRegistry`](crate::level::PrefabRegistry) of `Prefab` fragments, keyed
//! by a stable [`ThemeUuid`](crate::level::ThemeUuid) (GTW-485 / GTW-488).
//!
//! `mod.rs` is wiring-only: [`place`] holds the placement value types + the two placement
//! steps; [`pick`] holds the deterministic candidate scan + the two selectors.

mod pick;
mod place;

pub use place::{PlacedPrefab, Placement, assemble_placement, assemble_placement_with};
pub(in crate::lifecycle::procgen) use place::{place_enemy, place_player};
