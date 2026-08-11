use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{InspectTarget, SelectedShooter, contextual::ThrowGrenadeAct};
use gdtf_battle_sim::{
    acts::throw_grenade_tu_cost,
    ganger::{Faction, Position, Tu},
    prelude::CellLevel,
    tu::can_spend_tu,
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
    styles:  Query<'w, 's, &'static TrajectoryStyle>,
}

impl ThrowReads<'_, '_> {
    fn throw_target(&self, actor: Entity) -> Option<CellLevel> {
        let hovered = self.inspect.as_ref()?.hovered()?;
        let wields = self.wields.get(actor).ok()?;
        let weapon = wields.ranged_weapon(|entity| self.melee.get(entity).is_ok())?;
        let style = self.styles.get(weapon).ok()?;
        (*style.is_arc()).then_some(hovered)
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
    let actor = (**selected).and_then(|actor| actors.get(actor).ok().map(|tu| (actor, tu)));
    let target = actor.and_then(|(actor, _)| throw.throw_target(actor));
    let pressable = OfferPressable::new(
        actor
            .and_then(|(_, tu)| tu)
            .zip(tuning.as_deref())
            .is_some_and(|(tu, tuning)| *can_spend_tu(tu, throw_grenade_tu_cost(tuning))),
    );
    offer.set_if_neq(ContextualOffer::new(target).with_pressable(pressable));
}
