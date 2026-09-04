use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ExecuteAct};
use gdtf_battle_sim::{
    acts::downed::{Actor, DownedTarget, can_execute, execute_tu_cost},
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position, Tu},
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that executes an adjacent downed ganger.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExecuteButton;
}

impl ContextualPanelAct for ExecuteAct {
    type Marker = ExecuteButton;

    const SLOT: PanelSlot = PanelSlot::new(0);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Execute")
    }
}

type ExecuteCandidates = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
    Option<&'static BleedingOut>,
);

type ExecuteActorReads = (
    &'static Position,
    &'static LifeState,
    &'static Faction,
    Option<&'static Tu>,
);

pub(in crate::states::running::game::battlescape) fn offer_execute(
    selected: Res<SelectedShooter>,
    actors: Query<ExecuteActorReads>,
    candidates: Query<ExecuteCandidates>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<ExecuteAct>>,
) {
    let scanned = (**selected)
        .and_then(|entity| actors.get(entity).ok())
        .zip(tuning.as_deref())
        .and_then(|((pos, life, faction, pool), tuning)| {
            let actor = Actor {
                pos:     *pos,
                life:    *life,
                faction: *faction,
            };
            scan_execute(&actor, pool, &candidates, tuning)
        });
    let (target, pressable) = scanned
        .map_or((None, OfferPressable::new(false)), |(entity, pressable)| {
            (Some(entity), pressable)
        });
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

fn scan_execute(
    actor: &Actor,
    pool: Option<&Tu>,
    candidates: &Query<ExecuteCandidates>,
    tuning: &CombatTuning,
) -> Option<(Entity, OfferPressable)> {
    let allowed = |target: &DownedTarget, tu: &Tu| *can_execute(actor, target, tu, tuning);
    // Asking with the cost as the pool holds affordability true, so the other terms pick the target.
    let cost = execute_tu_cost(tuning);
    let (entity, target) = candidates
        .iter()
        .map(|(entity, pos, life, faction, bleeding_out)| {
            (
                entity,
                DownedTarget {
                    pos:          *pos,
                    life:         *life,
                    faction:      *faction,
                    bleeding_out: bleeding_out.copied(),
                },
            )
        })
        .find(|(_, target)| allowed(target, &cost))?;
    Some((
        entity,
        OfferPressable::new(pool.is_some_and(|tu| allowed(&target, tu))),
    ))
}
