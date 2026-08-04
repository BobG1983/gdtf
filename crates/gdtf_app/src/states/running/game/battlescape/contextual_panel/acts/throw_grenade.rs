use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{InspectTarget, SelectedShooter, contextual::ThrowGrenadeAct};
use gdtf_battle_sim::{
    ganger::{Faction, Position},
    prelude::CellLevel,
    weapon::{MeleeWeapon, TrajectoryStyle, Wields},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
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

pub(in crate::states::running::game::battlescape) fn offer_throw_grenade(
    selected: Res<SelectedShooter>,
    actors: Query<(), (With<Position>, With<Faction>)>,
    throw: ThrowReads,
    mut offer: ResMut<ContextualOffer<ThrowGrenadeAct>>,
) {
    let target = (**selected)
        .filter(|actor| actors.get(*actor).is_ok())
        .and_then(|actor| throw.throw_target(actor));
    offer.set_if_neq(ContextualOffer::new(target));
}
