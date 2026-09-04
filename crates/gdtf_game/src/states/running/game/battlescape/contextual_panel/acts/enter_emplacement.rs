use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::EnterEmplacementAct};
use gdtf_battle_sim::{
    acts::{can_enter_emplacement, enter_emplacement_tu_cost},
    emplacement::{EmplacementEntrySides, EmplacementFacing, EmplacementState},
    entity::TerrainCell,
    ganger::{Faction, Position, Tu},
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that mounts an emplacement from one of its rotated entry sides.
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

type EnterEmplacementReads = (
    Entity,
    &'static EmplacementState,
    &'static TerrainCell,
    Option<&'static EmplacementEntrySides>,
    Option<&'static EmplacementFacing>,
);

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
    let allowed =
        |seat: Position,
         state: &EmplacementState,
         sides: Option<&EmplacementEntrySides>,
         facing: Option<&EmplacementFacing>,
         tu: &Tu| { *can_enter_emplacement(actor, seat, state, sides, facing, tu, tuning) };
    // Asking with the cost as the pool holds affordability true, so the other terms pick the target.
    let cost = enter_emplacement_tu_cost(tuning);
    let (entity, seat, state, sides, facing) = emplacements
        .iter()
        .map(|(entity, state, cell, sides, facing)| {
            (entity, Position::new(**cell), state, sides, facing)
        })
        .find(|&(_, seat, state, sides, facing)| allowed(seat, state, sides, facing, &cost))?;
    Some((
        entity,
        OfferPressable::new(pool.is_some_and(|tu| allowed(seat, state, sides, facing, tu))),
    ))
}
