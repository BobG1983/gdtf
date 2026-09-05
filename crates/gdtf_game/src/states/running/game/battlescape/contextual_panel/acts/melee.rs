use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{SelectedShooter, contextual::MeleeAct};
use gdtf_battle_sim::{
    acts::{MeleeAttacker, MeleeReach, MeleeTarget, can_melee, melee_tu_cost, structure_stands},
    cover::CoverLedger,
    ganger::{Facing, Faction, LifeState, Position, Stance, StanceKind, Tu},
    los::{Observer, PeekOffset, Target, has_los},
    march::MarchGrids,
    prelude::{Cell, CellLevel, OccupancyGrid},
    surface::SurfaceGrid,
    tu::can_spend_tu,
    tuning::CombatTuning,
    weapon::{FightMode, MeleeWeapon, Wields},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that swings at an adjacent target.
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

type MeleeActorReads = (
    &'static Position,
    &'static Faction,
    Option<&'static Stance>,
    Option<&'static Facing>,
);

type MeleeCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
    Option<&'static Stance>,
);

#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct LosGrids<'w> {
    occupancy: Option<Res<'w, OccupancyGrid>>,
    surface:   Option<Res<'w, SurfaceGrid>>,
    cover:     Option<Res<'w, CoverLedger>>,
    tuning:    Option<Res<'w, CombatTuning>>,
}

/// The TU pool and the wielded melee weapon one strike would be charged against.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct MeleeStrikeCost<'w, 's> {
    pools:  Query<'w, 's, &'static Tu>,
    wields: Query<'w, 's, &'static Wields>,
    melee:  Query<'w, 's, (), With<MeleeWeapon>>,
    modes:  Query<'w, 's, &'static FightMode>,
}

impl MeleeStrikeCost<'_, '_> {
    /// Whether `actor` can pay for one strike with the melee weapon it wields.
    fn affords(&self, actor: Entity) -> OfferPressable {
        let cost = self
            .wields
            .get(actor)
            .ok()
            .and_then(|wields| wields.melee_weapon(|entity| self.melee.contains(entity)))
            .and_then(|weapon| self.modes.get(weapon).ok())
            .map(melee_tu_cost);
        OfferPressable::new(
            self.pools
                .get(actor)
                .ok()
                .zip(cost)
                .is_some_and(|(tu, cost)| *can_spend_tu(tu, cost)),
        )
    }
}

pub(in crate::states::running::game::battlescape) fn offer_melee(
    selected: Res<SelectedShooter>,
    actors: Query<MeleeActorReads>,
    candidates: Query<MeleeCandidates>,
    grids: LosGrids,
    strike: MeleeStrikeCost,
    mut offer: ResMut<ContextualOffer<MeleeAct>>,
) {
    let target = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .and_then(|(actor_pos, actor_faction, actor_stance, actor_facing)| {
            let attacker = MeleeAttacker::new(*actor_pos, *actor_faction);
            let ganger = match (actor_stance, actor_facing) {
                (Some(actor_stance), Some(actor_facing)) => {
                    scan_melee_target(attacker, *actor_stance, *actor_facing, &candidates, &grids)
                }
                _ => None,
            };
            ganger
                .map(MeleeTarget::Ganger)
                .or_else(|| scan_melee_structure(attacker, &grids).map(MeleeTarget::Structure))
        });
    let pressable = (**selected).map_or(OfferPressable::new(false), |actor| strike.affords(actor));
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

fn scan_melee_target(
    attacker: MeleeAttacker,
    actor_stance: Stance,
    actor_facing: Facing,
    candidates: &Query<MeleeCandidates>,
    grids: &LosGrids,
) -> Option<Entity> {
    let (Some(occupancy), Some(surface), Some(cover), Some(tuning)) = (
        grids.occupancy.as_ref(),
        grids.surface.as_ref(),
        grids.cover.as_ref(),
        grids.tuning.as_ref(),
    ) else {
        return None;
    };

    let is_floored = |entity: Entity| {
        candidates
            .get(entity)
            .is_ok_and(|(_, _, life, ..)| !*life.is_active())
    };

    let standing = Stance::new(StanceKind::Standing);
    let actor_pos = attacker.position;
    for (entity, pos, life, faction, stance) in candidates {
        if !*can_melee(attacker, MeleeReach::ganger(*pos, *faction, *life)) {
            continue;
        }
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
            &observer,
            &target,
            MarchGrids {
                occupancy,
                surface,
                cover,
            },
            tuning,
            is_floored,
        ) {
            return Some(entity);
        }
    }
    None
}

fn scan_melee_structure(attacker: MeleeAttacker, grids: &LosGrids) -> Option<CellLevel> {
    let cover = grids.cover.as_ref()?;
    let occupancy = grids.occupancy.as_ref()?;

    let key = *attacker.position;
    let level = key.level();

    for dy in -1..=1 {
        for dx in -1..=1 {
            let at = CellLevel::new(Cell::new(key.x + dx, key.y + dy), level);
            let standing = structure_stands(cover, occupancy, at);
            if !*can_melee(attacker, MeleeReach::structure(at, standing)) {
                continue;
            }
            if let Some(entry) = cover.peek(&at)
                && !*entry.destroyed
            {
                return Some(at);
            }
        }
    }
    None
}
