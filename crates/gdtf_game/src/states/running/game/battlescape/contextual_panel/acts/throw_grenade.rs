use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{InspectTarget, SelectedShooter, contextual::ThrowGrenadeAct};
use gdtf_battle_sim::{
    acts::{can_throw_grenade, throw_grenade_tu_cost},
    ganger::{Faction, Position, Tu},
    magazine::Magazine,
    prelude::CellLevel,
    tuning::CombatTuning,
    weapon::{MeleeWeapon, TrajectoryStyle, Wields},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, OfferPressable, PanelSlot,
};

crate::support_item! {
    /// Contextual button that throws a grenade at the last inspected cell.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ThrowGrenadeButton;
}

impl ContextualPanelAct for ThrowGrenadeAct {
    type Marker = ThrowGrenadeButton;

    const SLOT: PanelSlot = PanelSlot::new(7);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Throw")
    }
}

#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct ThrowReads<'w, 's> {
    inspect: Option<Res<'w, InspectTarget>>,
    wields:  Query<'w, 's, &'static Wields>,
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
    arms:    Query<'w, 's, (&'static TrajectoryStyle, &'static Magazine)>,
}

impl ThrowReads<'_, '_> {
    fn hovered_cell(&self) -> Option<CellLevel> {
        self.inspect.as_ref()?.hovered()
    }

    fn can_throw(&self, actor: Entity, tu: Tu, tuning: &CombatTuning) -> bool {
        let Ok(wields) = self.wields.get(actor) else {
            return false;
        };
        let Some(weapon) = wields.ranged_weapon(|entity| self.melee.get(entity).is_ok()) else {
            return false;
        };
        let Ok((style, magazine)) = self.arms.get(weapon) else {
            return false;
        };
        *can_throw_grenade(*style, magazine, &tu, tuning)
    }
}

type ThrowActorFilter = (With<Position>, With<Faction>);

pub(in crate::states::running::game::battlescape) fn offer_throw_grenade(
    selected: Res<SelectedShooter>,
    actors: Query<Option<&Tu>, ThrowActorFilter>,
    throw: ThrowReads,
    tuning: Option<Res<CombatTuning>>,
    mut offer: ResMut<ContextualOffer<ThrowGrenadeAct>>,
) {
    let scanned = (**selected)
        .and_then(|actor| actors.get(actor).ok().map(|tu| (actor, tu)))
        .zip(tuning.as_deref())
        .and_then(|((actor, pool), tuning)| {
            scan_throw(&throw, actor, pool, tuning, offer.target())
        });
    let next = match scanned {
        Some((target, pressable)) => ContextualOffer::new(target)
            .with_offered(true)
            .with_pressable(pressable),
        None => ContextualOffer::new(None).with_pressable(OfferPressable::new(false)),
    };
    offer.set_if_neq(next);
}

fn scan_throw(
    throw: &ThrowReads,
    actor: Entity,
    pool: Option<&Tu>,
    tuning: &CombatTuning,
    last_target: Option<CellLevel>,
) -> Option<(Option<CellLevel>, OfferPressable)> {
    let cost = throw_grenade_tu_cost(tuning);
    if !throw.can_throw(actor, cost, tuning) {
        return None;
    }
    Some((
        throw.hovered_cell().or(last_target),
        OfferPressable::new(pool.is_some_and(|tu| throw.can_throw(actor, *tu, tuning))),
    ))
}
