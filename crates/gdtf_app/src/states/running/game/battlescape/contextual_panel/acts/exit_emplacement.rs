use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ExitEmplacementAct};
use gdtf_battle_sim::{
    acts::{can_exit_emplacement, exit_emplacement_tu_cost},
    emplacement::{EmplacementState, MountedBy},
    ganger::{Faction, Position, Tu},
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that dismounts the occupied emplacement.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExitEmplacementButton;
}

impl ContextualPanelAct for ExitEmplacementAct {
    type Marker = ExitEmplacementButton;

    const SLOT: PanelSlot = PanelSlot::new(6);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Exit")
    }
}

type ExitEmplacementActorFilter = (With<Position>, With<Faction>);

type ExitEmplacementReads = (Entity, &'static EmplacementState, &'static MountedBy);

pub(in crate::states::running::game::battlescape) fn offer_exit_emplacement(
    selected: Res<SelectedShooter>,
    actors: Query<Option<&Tu>, ExitEmplacementActorFilter>,
    emplacements: Query<ExitEmplacementReads>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<ExitEmplacementAct>>,
) {
    let scanned = (**selected)
        .and_then(|actor| actors.get(actor).ok().map(|tu| (actor, tu)))
        .zip(tuning.as_deref())
        .and_then(|((actor, pool), tuning)| {
            scan_exit_emplacement(actor, pool, &emplacements, tuning)
        });
    let (target, pressable) = scanned
        .map_or((None, OfferPressable::new(false)), |(entity, pressable)| {
            (Some(entity), pressable)
        });
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

fn scan_exit_emplacement(
    actor: Entity,
    pool: Option<&Tu>,
    emplacements: &Query<ExitEmplacementReads>,
    tuning: &CombatTuning,
) -> Option<(Entity, OfferPressable)> {
    let allowed = |state: &EmplacementState, occupant: &MountedBy, tu: &Tu| {
        *can_exit_emplacement(actor, state, occupant, tu, tuning)
    };
    // Asking with the cost as the pool holds affordability true, so the other terms pick the target.
    let cost = exit_emplacement_tu_cost(tuning);
    let (entity, state, occupant) = emplacements
        .iter()
        .find(|&(_, state, occupant)| allowed(state, occupant, &cost))?;
    Some((
        entity,
        OfferPressable::new(pool.is_some_and(|tu| allowed(state, occupant, tu))),
    ))
}
