use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::EnterEmplacementAct};
use gdtf_battle_sim::{
    acts::{can_enter_emplacement, enter_emplacement_tu_cost},
    emplacement::EmplacementState,
    entity::TerrainCell,
    ganger::{Faction, Position, Tu},
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that mounts an adjacent emplacement.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EnterEmplacementButton;
}

impl ContextualPanelAct for EnterEmplacementAct {
    type Marker = EnterEmplacementButton;

    const SLOT: PanelSlot = PanelSlot::new(5);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Enter")
    }
}

type EnterEmplacementReads = (Entity, &'static EmplacementState, &'static TerrainCell);

type EnterEmplacementActorReads = (&'static Position, Option<&'static Tu>);

pub(in crate::states::running::game::battlescape) fn offer_enter_emplacement(
    selected: Res<SelectedShooter>,
    actors: Query<EnterEmplacementActorReads, With<Faction>>,
    emplacements: Query<EnterEmplacementReads>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<EnterEmplacementAct>>,
) {
    let scanned = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .zip(tuning.as_deref())
        .and_then(|((actor_pos, pool), tuning)| {
            scan_enter_emplacement(*actor_pos, pool, &emplacements, tuning)
        });
    let (target, pressable) = scanned
        .map_or((None, OfferPressable::new(false)), |(entity, pressable)| {
            (Some(entity), pressable)
        });
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

fn scan_enter_emplacement(
    actor: Position,
    pool: Option<&Tu>,
    emplacements: &Query<EnterEmplacementReads>,
    tuning: &CombatTuning,
) -> Option<(Entity, OfferPressable)> {
    let allowed = |seat: Position, state: &EmplacementState, tu: &Tu| {
        *can_enter_emplacement(actor, seat, state, tu, tuning)
    };
    // Asking with the cost as the pool holds affordability true, so the other terms pick the target.
    let cost = enter_emplacement_tu_cost(tuning);
    let (entity, seat, state) = emplacements
        .iter()
        .map(|(entity, state, cell)| (entity, Position::new(**cell), state))
        .find(|&(_, seat, state)| allowed(seat, state, &cost))?;
    Some((
        entity,
        OfferPressable::new(pool.is_some_and(|tu| allowed(seat, state, tu))),
    ))
}
