//! The typed [`BattleSetupError`] — the no-panic setup-abort contract.

use crate::{
    armor::ArmorName,
    ganger::{GangName, GangerName},
    metric::CellLevel,
    terrain::def::TerrainUuid,
    tuning::MoveCost,
    vertical::InvalidVerticalLink,
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
/// armor file supplies, the GTW-396 / GTW-491 variants
/// [`TerrainNotFound`](BattleSetupError::TerrainNotFound) (for a cover/slab terrain
/// definition UUID absent from the
/// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry))
/// and [`FloorCostBelowMinimum`](BattleSetupError::FloorCostBelowMinimum) (the GTW-482-bound
/// floor-cost-validation variant, not produced this slice),
/// and the GTW-457 [`StackedGangers`](BattleSetupError::StackedGangers) variant
/// (for two authored gangers sharing one `(cell, level)` spawn slot).
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
    /// A [`PlacedGanger`](crate::situation::PlacedGanger)'s
    /// [`gang`](crate::situation::PlacedGanger::gang) ref was not in the
    /// [`GangRegistry`](crate::ganger::GangRegistry) — no `assets/content/gangs/*.gang.ron`
    /// with that filename stem loaded (GTW-414/415). Validated BEFORE any entity is
    /// spawned (abort-first invariant).
    GangNotFound {
        /// The unresolved gang ref (the missing gang file's stem).
        gang: GangName,
    },
    /// A [`PlacedGanger`](crate::situation::PlacedGanger)'s
    /// [`member`](crate::situation::PlacedGanger::member) ref was not in its (resolved)
    /// gang's roster — the gang loaded, but no [`GangMember`](crate::ganger::GangMember)
    /// in it carries that [`GangerName`] (GTW-414/415). Validated BEFORE any entity is
    /// spawned (abort-first invariant).
    GangMemberNotFound {
        /// The gang the member was sought in (it DID resolve — only the member is missing).
        gang:   GangName,
        /// The unresolved member name (no roster member of `gang` carries it).
        member: GangerName,
    },
    /// A ganger's weapon key was not in the
    /// [`WeaponRegistry`](crate::weapon::WeaponRegistry) — no `assets/content/weapons/ranged/*.ron`
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
    /// A ganger's MELEE weapon key — its authored
    /// [`melee_weapon`](crate::ganger::GangMember::melee_weapon) key, or (when it authored
    /// none) the [`fists`](crate::weapon::FISTS_KEY) default — was not in the
    /// [`MeleeWeaponRegistry`](crate::weapon::MeleeWeaponRegistry): no
    /// `assets/content/weapons/melee/*.melee_weapon.ron` with that filename stem loaded
    /// (GTW-505, the melee mirror of [`WeaponNotFound`](BattleSetupError::WeaponNotFound)).
    /// Since EVERY ganger gets a melee weapon (an authored one OR `fists`), a missing
    /// `fists.melee_weapon.ron` would surface here for an un-authored ganger — fail-closed,
    /// exactly like the ranged path.
    MeleeWeaponNotFound {
        /// The unresolved melee weapon key (the missing file's stem, or `fists`).
        weapon: WeaponName,
    },
    /// A cover or slab terrain definition UUID was not in the
    /// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry) — no migrated
    /// [`TerrainDef`](crate::terrain::def::TerrainDef) with that
    /// [`TerrainUuid`] key loaded (GTW-491 — keyed by `TerrainUuid`, the successor to
    /// the GTW-396 filename-stem key).
    ///
    /// Validated BEFORE any entity is spawned (abort-first invariant), so a missing
    /// terrain UUID aborts the whole setup with no partial world behind.
    TerrainNotFound {
        /// The unresolved terrain definition UUID (no `TerrainDef` carries this key).
        piece: TerrainUuid,
    },
    /// An authored floor piece's move cost is below the A\* heuristic admissibility
    /// floor ([`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST) = 4) — which would
    /// make the heuristic inadmissible and produce silently wrong paths (GTW-396
    /// Decision B). Validated BEFORE any entity is spawned (abort-first).
    ///
    /// GTW-491: NOT produced this slice — the new [`TerrainSimKind`](crate::terrain::def::TerrainSimKind)
    /// model carries no per-piece floor move cost (the move-cost-from-`default_floor` seam is
    /// GTW-482), so floor cost is the uniform `fallback_floor_cost`. The variant is retained for
    /// the GTW-482 seam that will re-introduce per-floor move-cost validation.
    FloorCostBelowMinimum {
        /// The terrain piece whose `move_cost` is too low.
        piece:   TerrainUuid,
        /// The authored `move_cost` that was rejected.
        cost:    MoveCost,
        /// The minimum admissible cost (always [`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST)).
        minimum: MoveCost,
    },
    /// Two or more authored gangers share the SAME `(cell, level)` spawn slot
    /// (GTW-457). In-battle movement enforces single-occupancy
    /// (`docs/combat/resolution.md`: one object per cell), but the GTW-156
    /// occupancy pour is last-write-wins, so a duplicated authored cell would
    /// silently overwrite the first ganger's occupancy slot while BOTH entities
    /// survive standing on the same cell — a stacked, invalid world.
    ///
    /// This fails CLOSED before any ganger entity is spawned (abort-first): a
    /// trusted authored situation with stacked spawns is a DATA bug to be fixed,
    /// not auto-relocated (auto-relocation belongs in the GTW-424 procgen
    /// assembler, not here).
    ///
    /// Fix: give each ganger a distinct `at` `(cell, level)` in the authored
    /// situation `.ron`.
    StackedGangers {
        /// The `(cell, level)` two or more gangers were authored to share (the
        /// first duplicate detected).
        at: CellLevel,
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

impl std::fmt::Display for BattleSetupError {
    /// Render a human-readable, fail-closed setup error for the
    /// [`setup_battle_on_request`](crate::battle::setup_battle_on_request)
    /// `error!` log — each arm names the offending authored value so the bad
    /// situation `.ron` can be fixed. The [`InvalidLink`](BattleSetupError::InvalidLink)
    /// arm renders its `InvalidVerticalLink` via `Debug` (that type carries no
    /// `Display`), all other arms are bespoke messages.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLink(invalid) => write!(f, "invalid vertical link: {invalid:?}"),
            Self::GangNotFound { gang } => {
                write!(f, "no gang `{}` is loaded (missing gang file)", **gang)
            }
            Self::GangMemberNotFound { gang, member } => {
                write!(f, "gang `{}` has no member `{}`", **gang, **member)
            }
            Self::WeaponNotFound { weapon } => {
                write!(f, "no weapon `{}` is loaded", **weapon)
            }
            Self::ArmorNotFound { armor } => write!(f, "no armor `{}` is loaded", **armor),
            Self::MeleeWeaponNotFound { weapon } => {
                write!(f, "no melee weapon `{}` is loaded", **weapon)
            }
            Self::TerrainNotFound { piece } => {
                write!(f, "no terrain piece `{}` is loaded", **piece)
            }
            Self::FloorCostBelowMinimum {
                piece,
                cost,
                minimum,
            } => write!(
                f,
                "floor piece `{}` move cost {} is below the minimum {}",
                **piece, **cost, **minimum
            ),
            Self::StackedGangers { at } => write!(
                f,
                "two or more gangers are authored on the same spawn cell {:?} \
                 (each ganger needs a distinct (cell, level))",
                **at
            ),
        }
    }
}
