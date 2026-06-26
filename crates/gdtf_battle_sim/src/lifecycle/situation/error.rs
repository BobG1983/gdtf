//! The typed [`BattleSetupError`] — the no-panic setup-abort contract.

use crate::{
    armor::ArmorName, terrain::piece::TerrainName, tuning::MoveCost, vertical::InvalidVerticalLink,
    weapon::WeaponName,
};

/// The typed ways [`setup_battle`](crate::situation::setup_battle) can fail — the
/// no-panic setup-abort contract (GTW-257).
///
/// A named domain enum (no-bare-types: a setup failure is a domain value, not a
/// bare string / `()`), returned in the `Err` arm of
/// [`setup_battle`](crate::situation::setup_battle)'s [`Result`].
/// It subsumes the prior `Err(InvalidVerticalLink)` (now the
/// [`InvalidLink`](BattleSetupError::InvalidLink) variant), adds the GTW-257
/// [`WeaponNotFound`](BattleSetupError::WeaponNotFound) variant for a
/// [`GangerSpawn::weapon`](crate::situation::GangerSpawn::weapon) key that no loaded
/// weapon file supplies, the GTW-269
/// [`ArmorNotFound`](BattleSetupError::ArmorNotFound) variant for a
/// [`GangerSpawn::armor`](crate::situation::GangerSpawn::armor) key that no loaded
/// armor file supplies, and the GTW-396 variants
/// [`TerrainNotFound`](BattleSetupError::TerrainNotFound) (for a cover/slab/floor
/// key absent from the [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry))
/// and [`FloorCostBelowMinimum`](BattleSetupError::FloorCostBelowMinimum) (for a
/// floor piece whose move cost is below the A\* heuristic admissibility floor).
/// The caller
/// ([`setup_battle_on_request`](crate::battle::setup_battle_on_request)) matches on
/// it and fails closed (logs, no [`BattleReady`](crate::battle::BattleReady)) — it
/// NEVER panics / unwraps.
///
/// All variants are validated BEFORE any entity is spawned or any resource inserted
/// (the abort-first invariant), so a failure leaves no partial, unspawnable world
/// behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleSetupError {
    /// An authored vertical link failed validation (level out of range, dangling
    /// endpoint, or same-storey) — the prior `Err(InvalidVerticalLink)`, now wrapped.
    InvalidLink(InvalidVerticalLink),
    /// A ganger's [`weapon`](crate::situation::GangerSpawn::weapon) key was not in the
    /// [`WeaponRegistry`](crate::weapon::WeaponRegistry) — no `assets/content/weapons/*.ron`
    /// with that filename stem loaded.
    WeaponNotFound {
        /// The unresolved weapon key (the missing file's stem).
        weapon: WeaponName,
    },
    /// A ganger's [`armor`](crate::situation::GangerSpawn::armor) key was not in the
    /// [`ArmorRegistry`](crate::armor::ArmorRegistry) — no `assets/content/armor/*.armor.ron`
    /// with that filename stem loaded (the armor mirror of
    /// [`WeaponNotFound`](BattleSetupError::WeaponNotFound)).
    ArmorNotFound {
        /// The unresolved armor key (the missing file's stem).
        armor: ArmorName,
    },
    /// A cover, slab, or floor piece KEY was not in the
    /// [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry) — no
    /// `assets/content/terrain/*.terrain.ron` with that filename stem loaded (GTW-396).
    ///
    /// Validated BEFORE any entity is spawned (abort-first invariant), so a missing
    /// terrain key aborts the whole setup with no partial world behind.
    TerrainNotFound {
        /// The unresolved terrain key (the missing file's stem).
        piece: TerrainName,
    },
    /// An authored floor piece's move cost is below the A\* heuristic admissibility
    /// floor ([`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST) = 4) — which would
    /// make the heuristic inadmissible and produce silently wrong paths (GTW-396
    /// Decision B). Validated BEFORE any entity is spawned (abort-first).
    ///
    /// Fix: raise the floor piece's `move_cost` in its `assets/content/terrain/*.terrain.ron`
    /// to at least `MIN_MOVE_COST` (4). The `.ron` comment documents this constraint.
    FloorCostBelowMinimum {
        /// The terrain piece whose `move_cost` is too low.
        piece:   TerrainName,
        /// The authored `move_cost` that was rejected.
        cost:    MoveCost,
        /// The minimum admissible cost (always [`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST)).
        minimum: MoveCost,
    },
}

impl From<InvalidVerticalLink> for BattleSetupError {
    /// Lift a vertical-link validation failure into a setup error — so
    /// [`setup_battle`](crate::situation::setup_battle) can `?`-propagate
    /// [`build_vertical_link_graph`](crate::vertical::build_vertical_link_graph)'s
    /// error straight into its own [`BattleSetupError`] result.
    fn from(invalid: InvalidVerticalLink) -> Self {
        Self::InvalidLink(invalid)
    }
}
