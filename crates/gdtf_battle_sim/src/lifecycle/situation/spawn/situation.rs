//! [`Situation`] — the canonical authored battlefield aggregate plus its
//! authored-cells iterator.

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::{
    piece_spawns::{CoverSpawn, FieldSpawn, FloorSpawn, SlabSpawn},
    placed_ganger::PlacedGanger,
};
use crate::{
    ganger::Faction,
    level::{GridSize, ThemeUuid},
    metric::CellLevel,
    terrain::def::TerrainUuid,
    vertical::VerticalLink,
};

/// The canonical authored **situation** — the full generated / authored
/// battlefield the battle is built from: gangers, walls, scatter, upper-floor
/// slabs, stair/ladder vertical links, and the floor cost surface (this
/// [`crate::situation`] module is the setup-on-entry source of truth).
///
/// This is the ONE situation type: it supersedes the GTW-156 placeholder (now
/// [`crate::occupancy::OccupancyInput`], the grid construction input) and GTW-160's
/// `vertical_links` extension (moved here). [`setup_battle`](crate::situation::setup_battle)
/// reads it to build the battle in the ECS world.
///
/// GTW-396 migration (schema change): `walls`/`scatter` entries now carry a terrain
/// piece KEY ([`CoverSpawn::piece`]) instead of inline stats; `slabs` entries are now
/// [`SlabSpawn`] structs carrying a piece KEY alongside their `at` cell (replacing the
/// old `Vec<CellLevel>` bare list); `default_floor` names the walkable floor piece for
/// all open cells; `floors` gives sparse per-cell floor overrides.
///
/// # The roster-vs-placement contract (GTW-414 schema v2)
///
/// The v2 schema SPLITS what an authored ganger carries between the **gang roster** (the
/// reusable, faction-agnostic identity + abilities + equipment) and the **situation**
/// (where + which side it fights on):
///
/// - **Lives in the gang roster** ([`GangMember`](crate::ganger::GangMember), in a
///   `*.gang.ron`, resolved via the [`GangRegistry`](crate::ganger::GangRegistry)): the
///   member's [`GangerName`](crate::ganger::GangerName) identity, its EIGHT direct attributes (Speed / Aim /
///   Strength / Toughness / Reflexes / Cool / Grit / Luck), its weapon KEY, and its armor
///   KEY. A gang roster says NOTHING about position or faction, so the SAME roster is
///   reusable in any battle, on any side, at any cell.
/// - **The situation assigns** ([`PlacedGanger`], in this `Situation`): WHICH gang +
///   member fights ([`gang`](PlacedGanger::gang) + [`member`](PlacedGanger::member)
///   refs), WHERE ([`at`](PlacedGanger::at) + [`facing`](PlacedGanger::facing) /
///   [`stance`](PlacedGanger::stance) / [`aiming`](PlacedGanger::aiming) /
///   [`life_state`](PlacedGanger::life_state)), and WHICH SIDE
///   ([`faction`](PlacedGanger::faction) — faction is placement/assignment, NOT roster).
///
/// [`setup_battle`](crate::situation::setup_battle) joins the two: for each
/// [`PlacedGanger`] it resolves `(gang, member)` against the
/// [`GangRegistry`](crate::ganger::GangRegistry) to get the roster member, derives the
/// computed combat stats from its eight attributes, resolves its weapon / armor keys, and
/// spawns the ganger at the situation-supplied placement + faction.
///
/// Derives [`Deserialize`] (GTW-205 / E10.3) so an authored battlefield ships as a
/// loose `.ron` file loaded through the `RonAsset<T>` loader — render-free and
/// pixel-free, the whole value graph routed through the landed newtype/enum serde
/// derives. `#[serde(default)]` on each list lets an authored file omit a section it
/// does not use (an empty battlefield deserializes from `()`), matching the [`Default`]
/// empty situation. The in-test fixture is a test helper; the shipped `.ron` source is
/// the real input.
///
/// Derives [`TypePath`] (render-free reflection metadata, no rendering) because the
/// `RonAsset<Situation>` the loader wraps it in requires
/// its payload to be [`TypePath`] — the same bound the theme spec satisfies.
#[derive(Debug, Clone, Default, Deserialize, TypePath)]
#[serde(default)]
pub struct Situation {
    /// The authored placed gangers (GTW-414 schema v2), each a [`PlacedGanger`]: a gang +
    /// member REF (resolved against the [`GangRegistry`](crate::ganger::GangRegistry) at
    /// setup for identity / attributes / equipment) plus the situation-supplied placement
    /// (`at` / `facing` / `stance` / `aiming` / `life_state`) and `faction`.
    pub gangers:        Vec<PlacedGanger>,
    /// The level THEME this battlefield draws from — the stable UUID-keyed reference to a
    /// migrated [`UuidThemeDef`](crate::level::UuidThemeDef), resolvable in the
    /// [`UuidThemeRegistry`](crate::level::UuidThemeRegistry).
    ///
    /// GTW-491 migration (T07a): switched from the closed-enum theme model to the UUID-keyed
    /// [`ThemeUuid`] (reconciling the GTW-490 additive
    /// `theme_uuid` field — the canonical sim theme is now this one `theme`). `#[serde(default)]`
    /// supplies [`ThemeUuid::default`] (the nil sentinel) for any authored file that omits the
    /// field. An authored `theme: "<uuid>"` parses the `#[serde(transparent)]` [`ThemeUuid`]
    /// string wire form.
    pub theme:          ThemeUuid,
    /// The coarse GRID dimensions of this battlefield (GTW-414) — width × height ×
    /// storey-count, each axis validated to the sim's coarse-grid maximum (GTW-409
    /// [`GridSize`]).
    ///
    /// `#[serde(default)]` supplies [`GridSize::default`] = the FULL documented `60×60×8`
    /// extent for any authored file that omits the field, so every PRE-GTW-414 situation
    /// `.ron` stays parse-valid — the full sim extent is the safe superset of any cell a
    /// pre-GTW-414 file authored. An authored
    /// `grid_size: (width: N, height: N, levels: N)` flows through the bounds-checked
    /// [`GridSize`] serde intermediate, so an over-max / empty grid can never deserialize.
    pub grid_size:      GridSize,
    /// The authored walls (each a [`CoverSpawn`] — `at` + piece KEY).
    pub walls:          Vec<CoverSpawn>,
    /// The authored scatter / props (each a [`CoverSpawn`], same schema as a wall).
    pub scatter:        Vec<CoverSpawn>,
    /// The authored floor / roof slabs (each a [`SlabSpawn`] — `at` + piece KEY).
    /// GTW-396: was `Vec<CellLevel>`; now `Vec<SlabSpawn>` so each slab carries its
    /// terrain piece KEY for per-slab HP/armor resolution against the
    /// terrain-definition registry.
    pub slabs:          Vec<SlabSpawn>,
    /// The authored stair / ladder vertical links (E1.10 / GTW-160), moved here
    /// from the GTW-156 placeholder. The only way a ganger changes storey
    /// (`docs/combat/combat.md`).
    pub vertical_links: Vec<VerticalLink>,
    /// The gang the human player controls — every other [`Faction`] is the enemy.
    /// Seeds the [`PlayerFaction`](crate::battle::PlayerFaction) battle-lifetime resource the
    /// later control-gating + victory-census slices read. The struct-level
    /// `#[serde(default)]` supplies [`Faction::default`] = `Faction(0)` for any
    /// authored file that omits the field, so every existing situation `.ron` stays
    /// valid (gang `0` is the player by convention, matching `skirmish.ron`); an
    /// authored `player_faction: 1` parses as the bare gang index
    /// ([`Faction`] is `#[serde(transparent)]`).
    pub player_faction: Faction,
    /// The default floor terrain definition KEY — the stable [`TerrainUuid`] of a migrated
    /// walkable [`TerrainDef`](crate::terrain::def::TerrainDef) (a `Slab`-kind floor in the new
    /// model). Applied to every walkable open cell not overridden by [`floors`](Situation::floors).
    ///
    /// GTW-491 migration (T07a): switched from the legacy filename-stem
    /// [`TerrainName`](crate::terrain::piece::TerrainName) key to the UUID-keyed [`TerrainUuid`].
    /// `#[serde(default)]` supplies the NIL sentinel ([`TerrainUuid::default`] —
    /// [`TerrainUuid::is_nil`]) for any authored file that omits the field, so every EXISTING
    /// situation `.ron` remains parse-valid. A nil default skips registry floor resolution (the
    /// sim falls back to seeding the [`FloorCostGrid`](crate::terrain::floor::FloorCostGrid) from
    /// `CombatTuning::move_costs.open`, preserving the pre-GTW-396 behavior for test fixtures and
    /// un-migrated situations).
    pub default_floor:  TerrainUuid,
    /// Sparse per-cell floor piece overrides — cells whose floor differs from
    /// [`default_floor`](Situation::default_floor).
    /// `#[serde(default)]` gives an empty list (the common case: uniform floor).
    pub floors:         Vec<FloorSpawn>,
    /// The authored area-damage **field placements** (GTW-545) — each a [`FieldSpawn`]: a
    /// `(cell, level)` paired with a field-type KEY resolved against the
    /// [`FieldDefRegistry`](crate::effects::fields::FieldDefRegistry) and seeded into the live
    /// [`FieldRegistry`](crate::effects::fields::FieldRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) (e.g. a toxic-waste pool as initial
    /// terrain). `#[serde(default)]` gives an empty list, so every EXISTING situation `.ron`
    /// deserializes byte-identical (a battlefield with no hazards omits the field entirely).
    pub fields:         Vec<FieldSpawn>,
}

impl Situation {
    /// Build an empty situation (no gangers, cover, slabs, or links).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every `(cell, level)` this situation **authors** a tile at — the union of
    /// its wall, scatter, and slab cells.
    ///
    /// This is the **cell-existence source** for vertical-link validation
    /// ([`build_vertical_link_graph`](crate::vertical::build_vertical_link_graph)):
    /// a link endpoint that does not appear here
    /// dangles off a `(cell, level)` no authored tile occupies. Gangers are NOT
    /// included — a ganger standing somewhere does not author a tile a link can
    /// attach to (`docs/combat/combat.md`: links attach to authored geometry).
    pub fn authored_cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.walls
            .iter()
            .map(|c| c.at)
            .chain(self.scatter.iter().map(|c| c.at))
            .chain(self.slabs.iter().map(|s| s.at))
    }
}
