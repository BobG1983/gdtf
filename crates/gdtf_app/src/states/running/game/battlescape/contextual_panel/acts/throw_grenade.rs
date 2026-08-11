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
    /// Contextual button that throws a grenade at the inspected cell.
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
    /// The hovered cell, when the sim would allow this thrower to lob at it with `tu`.
    fn throw_target(&self, actor: Entity, tu: Tu, tuning: &CombatTuning) -> Option<CellLevel> {
        let hovered = self.inspect.as_ref()?.hovered()?;
        let wields = self.wields.get(actor).ok()?;
        let weapon = wields.ranged_weapon(|entity| self.melee.get(entity).is_ok())?;
        let (style, magazine) = self.arms.get(weapon).ok()?;
        (*can_throw_grenade(*style, magazine, &tu, tuning)).then_some(hovered)
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
        .and_then(|((actor, pool), tuning)| scan_throw(&throw, actor, pool, tuning));
    let (target, pressable) = scanned
        .map_or((None, OfferPressable::new(false)), |(target, pressable)| {
            (Some(target), pressable)
        });
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}

fn scan_throw(
    throw: &ThrowReads,
    actor: Entity,
    pool: Option<&Tu>,
    tuning: &CombatTuning,
) -> Option<(CellLevel, OfferPressable)> {
    // Asking with the cost as the pool holds affordability true, so the other terms pick the target.
    let cost = throw_grenade_tu_cost(tuning);
    let target = throw.throw_target(actor, cost, tuning)?;
    Some((
        target,
        OfferPressable::new(
            pool.is_some_and(|tu| throw.throw_target(actor, *tu, tuning).is_some()),
        ),
    ))
}
