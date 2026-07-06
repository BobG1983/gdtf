//! The **Melee** contextual act's panel-layer module (GTW-507 / GTW-508 / GTW-571):
//! marker, descriptor, and the two-kind offer scan (ganger strike, else cover-smash).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{SelectedShooter, contextual::MeleeAct};
use gdtf_battle_sim::{
    acts::{MeleeTarget, downed::is_8_adjacent},
    cover::CoverLedger,
    ganger::{Facing, Faction, LifeState, Position, Stance, StanceKind},
    los::{Observer, PeekOffset, Target, has_los},
    prelude::{Cell, CellLevel, OccupancyGrid},
    surface::SurfaceGrid,
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Melee** contextual button (GTW-507) — the close-combat strike act on an
    /// 8-adjacent, alive, in-LOS ENEMY ganger, or (GTW-508) the smash of an 8-adjacent
    /// intact Cover / Wall cell when no ganger is in reach (`docs/combat/resolution.md` §7).
    ///
    /// A DEDICATED contextual button (the GTW-507 D1 ruling — NOT a left-click overload).
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic
    /// button spawn and revealed IN PLACE by the act's visibility toggle when
    /// [`offer_melee`] names either target kind. A unit marker: presence on an entity is the
    /// whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MeleeButton;
}

impl ContextualPanelAct for MeleeAct {
    type Marker = MeleeButton;

    const SLOT: PanelSlot = PanelSlot::new(2);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Melee")
    }
}

/// The actor (selection) reads the melee scan needs — its grid cell + its gang
/// (required) plus its OPTIONAL stance + facing (the GTW-507 LOS observer eye). A named
/// alias for clippy `type_complexity`. [`Stance`] / [`Facing`] are [`Option`] so a
/// minimal actor that carries neither can still be offered the GTW-508 STRUCTURE smash
/// (which needs no LOS eye); the ganger strike simply offers nothing without both.
type MeleeActorReads = (
    &'static Position,
    &'static Faction,
    Option<&'static Stance>,
    Option<&'static Facing>,
);

/// The candidate-neighbour reads the melee scan needs — each ganger's identity, cell,
/// life, gang, and OPTIONAL stance (the LOS aim silhouette; a stance-less candidate
/// falls back to standing). A named alias for clippy `type_complexity`.
type MeleeCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
    Option<&'static Stance>,
);

/// The change-driven world grids + tuning the GTW-507 melee LOS gate reads, bundled
/// into one [`SystemParam`] so [`offer_melee`] stays under clippy's argument-count gate
/// (the sim's `BattleGridsParam` precedent).
///
/// The three grids [`has_los`] marches through (read-only — the offer layer never
/// mutates the sim) plus the [`CombatTuning`] the LOS geometry reads. `Option` reads so
/// the system stays valid before a battle inserts them (the melee scan then offers no
/// target — `bevy-traps.md` #1); in a live battle they are always present.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct LosGrids<'w> {
    /// The coarse 3D occupancy grid — the LOS march's collision / occupant-band surface
    /// (also the observer's stair-eye-offset lookup).
    occupancy: Option<Res<'w, OccupancyGrid>>,
    /// The persistent floor/roof-slab + ground surface grid the LOS march flies through.
    surface:   Option<Res<'w, SurfaceGrid>>,
    /// The model cover ledger — peeked (read only) for the LOS march's cover bands and
    /// the GTW-508 smash scan's intact-structure lookup.
    cover:     Option<Res<'w, CoverLedger>>,
    /// The combat tuning the LOS view geometry reads.
    tuning:    Option<Res<'w, CombatTuning>>,
}

/// OFFERS the Melee act: the first 8-adjacent, ALIVE, in-LOS ENEMY ganger
/// ([`MeleeTarget::Ganger`], GTW-507) — or, with NO meleeable ganger in reach, the
/// first 8-adjacent intact Cover / Wall cell to SMASH ([`MeleeTarget::Structure`],
/// GTW-508) — or nothing. The ganger strike takes priority, so the ONE Melee button
/// routes to a strike or a cover-smash, never both.
///
/// The LOS observer eye needs the actor's stance + facing; without BOTH (a minimal
/// actor) no ganger strike is offered — the structure smash, which needs neither, still
/// can be. The sim's `dispatch_melee` gate is the authoritative re-check when the act
/// fires; this reuses [`has_los`] verbatim so the offer matches the sim's geometry
/// truth. Writes [`ContextualOffer`] via `set_if_neq` (change-detection hygiene).
pub(in crate::states::running::game::battlescape) fn offer_melee(
    selected: Res<SelectedShooter>,
    actors: Query<MeleeActorReads>,
    candidates: Query<MeleeCandidates>,
    grids: LosGrids,
    mut offer: ResMut<ContextualOffer<MeleeAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction, actor_stance, actor_facing)| {
            let ganger = match (actor_stance, actor_facing) {
                (Some(actor_stance), Some(actor_facing)) => scan_melee_target(
                    *actor_pos,
                    *actor_faction,
                    *actor_stance,
                    *actor_facing,
                    &candidates,
                    &grids,
                ),
                _ => None,
            };
            ganger
                .map(MeleeTarget::Ganger)
                .or_else(|| scan_melee_structure(*actor_pos, &grids).map(MeleeTarget::Structure))
        });
    offer.set_if_neq(ContextualOffer::new(target));
}

/// Scans `candidates` for the actionable MELEE-GANGER target — the first 8-adjacent,
/// ALIVE, ENEMY ganger with a clear line of sight from the actor (GTW-507;
/// `docs/combat/resolution.md` §7).
///
/// A STRONGER offer gate than Execute's downed-adjacency: the target must be ALIVE
/// (incl. Downed — [`LifeState::is_active`]), of the OPPOSING faction, 8-adjacent
/// ([`is_8_adjacent`]), AND in clear LOS ([`has_los`], REUSED verbatim so the offered
/// strike matches the sim's geometry truth). Returns [`None`] when none qualifies —
/// including when the battle grids are absent (a pre-battle frame). A corpse never
/// blocks the LOS march (the `is_dead` pass-through reused). Pure over the queried
/// components + the read grids.
fn scan_melee_target(
    actor_pos: Position,
    actor_faction: Faction,
    actor_stance: Stance,
    actor_facing: Facing,
    candidates: &Query<MeleeCandidates>,
    grids: &LosGrids,
) -> Option<Entity> {
    // The melee LOS gate needs the live battle grids; with any absent (pre-battle)
    // offer nothing — fail-closed (bevy-traps.md #1: handle the Option, never unwrap).
    let (Some(occupancy), Some(surface), Some(cover), Some(tuning)) = (
        grids.occupancy.as_ref(),
        grids.surface.as_ref(),
        grids.cover.as_ref(),
        grids.tuning.as_ref(),
    ) else {
        return None;
    };

    // A Dead ganger is a corpse the LOS march flies THROUGH (the same `is_dead`
    // pass-through `has_los` takes). An absent entity is no corpse.
    let is_dead = |entity: Entity| {
        candidates
            .get(entity)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };

    // The fallback silhouette for a candidate carrying no stance (a real fielded ganger
    // always has one; this keeps the LOS aim defined for a minimal test/edge entity).
    let standing = Stance::new(StanceKind::Standing);
    for (entity, pos, life, faction, stance) in candidates {
        // An ALIVE (incl. Downed) OPPOSING ganger within the 8-adjacent reach.
        if !life.is_active() || *faction == actor_faction || !is_8_adjacent(actor_pos, *pos) {
            continue;
        }
        // The LOS gate — a clear sight line actor → target over the SAME voxel geometry
        // the sim fires through (built exactly as `dispatch_melee` builds them).
        let observer = Observer {
            position:         &actor_pos,
            stance:           &actor_stance,
            facing:           &actor_facing,
            stair_eye_offset: occupancy.stair_eye_offset_at(&actor_pos),
            peek_offset:      PeekOffset::default(),
        };
        let target = Target {
            position: pos,
            stance:   stance.unwrap_or(&standing),
        };
        if *has_los(
            &observer, &target, occupancy, surface, cover, tuning, is_dead,
        ) {
            return Some(entity);
        }
    }
    None
}

/// Scans the actor's 8 same-storey neighbour cells for an intact Cover / Wall cell to
/// SMASH — the first 8-adjacent cell whose [`CoverLedger`] entry stands (not destroyed)
/// (GTW-508; `docs/combat/resolution.md` §7 — the melee-smash of adjacent cover).
///
/// Returns [`None`] when no neighbour holds intact cover — including when the cover
/// ledger is absent (a pre-battle frame). A destroyed entry (already smashed) is
/// skipped — there is nothing left to hit. Uses [`CoverLedger::peek`] (the non-seeding
/// read) so only a REGISTERED (authored / already-hit) piece is offered — an unauthored
/// empty cell is not a structure. The scan order is the deterministic `(dy, dx)`
/// Moore-8 ring. Pure over the read cover ledger.
fn scan_melee_structure(actor_pos: Position, grids: &LosGrids) -> Option<CellLevel> {
    // The smash offer needs the live cover ledger; absent (pre-battle) → offer nothing.
    let cover = grids.cover.as_ref()?;

    // The actor's `(cell, level)` — the canonical CellLevel accessors through
    // Position's deref (GTW-565); the Moore-8 ring offsets from the key's x/y below.
    let key = *actor_pos;
    let level = key.level();

    // Scan the 8 same-storey Moore-neighbour cells in a deterministic (dy, dx) order;
    // offer the first that holds an intact (not-destroyed) REGISTERED cover entry.
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue; // the actor's own cell is not adjacent.
            }
            let at = CellLevel::new(Cell::new(key.x + dx, key.y + dy), level);
            if let Some(entry) = cover.peek(&at)
                && !*entry.destroyed
            {
                return Some(at);
            }
        }
    }
    None
}
