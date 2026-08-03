use bevy::{prelude::*, ui::Display};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_presenter::DrawnMagazine;
use gdtf_battle_sim::{
    magazine::Magazine,
    weapon::{MeleeWeapon, WeaponName, WieldedBy, Wields},
};

use crate::states::running::game::battlescape::weapon_panel::components::{
    ReloadButton, WeaponContent, WeaponMagazineText, WeaponNameText,
};

#[derive(bevy::ecs::query::QueryData)]
pub(in crate::states::running::game::battlescape::weapon_panel) struct WeaponData {
        name:     Option<&'static WeaponName>,
        magazine: Option<&'static Magazine>,
                    drawn:    Option<&'static DrawnMagazine>,
}

type ContentFilter = (
    With<WeaponContent>,
    Without<WeaponMagazineText>,
    Without<ReloadButton>,
);
type NameFilter = (With<WeaponNameText>, Without<WeaponMagazineText>);
type MagazineFilter = (
    With<WeaponMagazineText>,
    Without<WeaponContent>,
    Without<WeaponNameText>,
    Without<ReloadButton>,
);
type ReloadFilter = (
    With<ReloadButton>,
    Without<WeaponContent>,
    Without<WeaponMagazineText>,
);

#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape::weapon_panel) struct WeaponWidgets<'w, 's> {
            content:  Query<'w, 's, (&'static mut Visibility, &'static mut Node), ContentFilter>,
        name:     Query<'w, 's, &'static mut Text, NameFilter>,
        magazine: Query<'w, 's, (&'static mut Text, &'static mut Visibility), MagazineFilter>,
        reload:   Query<'w, 's, &'static mut Visibility, ReloadFilter>,
}

fn set_text(text: &mut Text, value: &str) {
    value.clone_into(&mut text.0);
}

fn set_visible(visibility: &mut Visibility, visible: bool) {
    let want = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != want {
        *visibility = want;
    }
}

fn set_content_shown(visibility: &mut Visibility, node: &mut Node, shown: bool) {
    set_visible(visibility, shown);
    let want = if shown { Display::Flex } else { Display::None };
    if node.display != want {
        node.display = want;
    }
}

pub(in crate::states::running::game::battlescape) fn update_weapon_panel(
    selected: Res<SelectedShooter>,
    wields: Query<&Wields>,
    data: Query<WeaponData, With<WieldedBy>>,
    melee: Query<(), With<MeleeWeapon>>,
    mut widgets: WeaponWidgets,
) {
    let selection = (**selected)
        .and_then(|ganger| wields.get(ganger).ok())
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .and_then(|weapon| data.get(weapon).ok());

    let (has_weapon, name, mag_line, has_magazine) = match selection {
        Some(item) => {
            let has_weapon = item.name.is_some() || item.magazine.is_some();
            let name = item.name.map(|n| (**n).clone()).unwrap_or_default();
            let magazine = item
                .drawn
                .map(DrawnMagazine::magazine)
                .or_else(|| item.magazine.copied());
            let (mag_line, has_magazine) = magazine.map_or((String::new(), false), |m| {
                let size = *m.size();
                if size > 0 {
                    (format!("{}/{}", *m.rounds(), size), true)
                } else {
                    (String::new(), false)
                }
            });
            (has_weapon, name, mag_line, has_magazine)
        }
        None => (false, String::new(), String::new(), false),
    };

    if let Ok((mut visibility, mut node)) = widgets.content.single_mut() {
        set_content_shown(&mut visibility, &mut node, has_weapon);
    }

    if let Ok(mut text) = widgets.name.single_mut() {
        set_text(&mut text, &name);
    }

    if let Ok((mut text, mut visibility)) = widgets.magazine.single_mut() {
        set_text(&mut text, &mag_line);
        set_visible(&mut visibility, has_magazine);
    }

    if let Ok(mut visibility) = widgets.reload.single_mut() {
        set_visible(&mut visibility, has_magazine);
    }
}
