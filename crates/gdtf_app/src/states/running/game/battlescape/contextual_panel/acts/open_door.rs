use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::OpenDoorAct};
use gdtf_battle_sim::{
    acts::{can_open_door, open_door_tu_cost},
    entity::TerrainCell,
    ganger::{Faction, Position, Tu},
    openable::OpenState,
    tuning::CombatTuning,
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that opens or closes an adjacent door.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OpenDoorButton;
}

impl ContextualPanelAct for OpenDoorAct {
    type Marker = OpenDoorButton;

    const SLOT: PanelSlot = PanelSlot::new(4);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Open Door")
    }
}

type DoorReads = (Entity, &'static OpenState, &'static TerrainCell);

type OpenDoorActorReads = (&'static Position, Option<&'static Tu>);

pub(in crate::states::running::game::battlescape) fn offer_open_door(
    selected: Res<SelectedShooter>,
    actors: Query<OpenDoorActorReads, With<Faction>>,
    doors: Query<DoorReads>,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<OpenDoorAct>>,
) {
    let scanned = (**selected)
        .and_then(|actor| actors.get(actor).ok())
        .zip(tuning.as_deref())
        .and_then(|((actor_pos, pool), tuning)| scan_open_door(*actor_pos, pool, &doors, tuning));
    let (target, pressable) = scanned
        .map_or((None, OfferPressable::new(false)), |(entity, pressable)| {
            (Some(entity), pressable)
        });
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

fn scan_open_door(
    actor: Position,
    pool: Option<&Tu>,
    doors: &Query<DoorReads>,
    tuning: &CombatTuning,
) -> Option<(Entity, OfferPressable)> {
    let allowed = |door: Position, open_state: OpenState, tu: &Tu| {
        *can_open_door(actor, door, open_state, tu, tuning)
    };
    // Asking with the cost as the pool holds affordability true, so the other terms pick the target.
    let cost = open_door_tu_cost(tuning);
    let (entity, door, open_state) = doors
        .iter()
        .map(|(entity, open_state, cell)| (entity, Position::new(**cell), *open_state))
        .find(|&(_, door, open_state)| allowed(door, open_state, &cost))?;
    Some((
        entity,
        OfferPressable::new(pool.is_some_and(|tu| allowed(door, open_state, tu))),
    ))
}
