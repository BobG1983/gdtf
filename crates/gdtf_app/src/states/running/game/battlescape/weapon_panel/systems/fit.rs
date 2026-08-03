use bevy::{
    prelude::*,
    text::{FontSize, TextFont},
    ui::{Node, Val},
};

use crate::states::running::game::battlescape::weapon_panel::components::{
    AimLabel, ReloadButton, WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText,
};

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ItemPadVw(f32);

const ITEM_PAD_VW: ItemPadVw = ItemPadVw(0.3125);

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct WeaponLabelPt(f32);

const WEAPON_LABEL_PT: WeaponLabelPt = WeaponLabelPt(14.0);

type WeaponLabelOwner = Or<(
    With<WeaponNameText>,
    With<WeaponMagazineText>,
    With<WeaponItemButton>,
    With<ReloadButton>,
    With<AimLabel>,
)>;

pub(in crate::states::running::game::battlescape) fn fit_weapon_panel(
    mut item_panels: Query<&mut Node, With<WeaponItemPanel>>,
    owners: Query<(Entity, Option<&Children>), WeaponLabelOwner>,
    mut fonts: Query<&mut TextFont, With<Text>>,
) {
    for mut node in &mut item_panels {
        if node.padding.left != Val::Vw(*ITEM_PAD_VW) {
            node.padding.left = Val::Vw(*ITEM_PAD_VW);
        }
        if node.padding.right != Val::Vw(*ITEM_PAD_VW) {
            node.padding.right = Val::Vw(*ITEM_PAD_VW);
        }
    }

    for (owner, children) in &owners {
        set_label_font(&mut fonts, owner);
        let Some(children) = children else {
            continue;
        };
        for &child in children {
            set_label_font(&mut fonts, child);
        }
    }
}

fn set_label_font(fonts: &mut Query<&mut TextFont, With<Text>>, label: Entity) {
    let Ok(mut font) = fonts.get_mut(label) else {
        return;
    };
    if font.font_size != FontSize::Px(*WEAPON_LABEL_PT) {
        font.font_size = FontSize::Px(*WEAPON_LABEL_PT);
    }
}
