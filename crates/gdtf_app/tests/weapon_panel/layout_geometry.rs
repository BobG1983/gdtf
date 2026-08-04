use std::path::PathBuf;

use bevy::{
    ecs::{entity::Entity, query::With},
    math::Vec2,
    prelude::*,
    text::TextLayoutInfo,
    ui::{ComputedNode, UiGlobalTransform},
};
use gdtf_app::test_support::{ReloadButton, WeaponContent, WeaponNameText};
use gdtf_battle_sim::{
    magazine::{Magazine, ReloadTu},
    weapon::MagazineSize,
};

use super::{
    harness::{spawn_armed_and_select, weapon_kit},
    real_layout_harness::real_layout_battle_running_app,
};

const RANGED_WEAPON_EXTENSION: &str = ".weapon.ron";

const EPSILON_PX: f32 = 1.0;

fn longest_shipped_ranged_weapon_name() -> String {
    let weapons_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("content")
        .join("weapons")
        .join("ranged");
    let mut longest = String::new();
    let Ok(entries) = std::fs::read_dir(&weapons_dir) else {
        return longest;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = file_name.strip_suffix(RANGED_WEAPON_EXTENSION) else {
            continue;
        };
        if stem.len() > longest.len() {
            stem.clone_into(&mut longest);
        }
    }
    longest
}

struct PixelRect {
    min: Vec2,
    max: Vec2,
}

impl PixelRect {
    fn from_node(node: &ComputedNode, transform: &UiGlobalTransform) -> Self {
        let size = node.size();
        let center = transform.translation;
        Self {
            min: center - size / 2.0,
            max: center + size / 2.0,
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

fn pixel_rect_of<M: Component>(app: &mut App) -> Option<PixelRect> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<M>>();
    let mut results: Vec<PixelRect> = query
        .iter(app.world())
        .map(|(node, transform)| PixelRect::from_node(node, transform))
        .collect();
    if results.len() == 1 {
        results.pop()
    } else {
        None
    }
}

fn weapon_name_text_entity(app: &mut App) -> Option<Entity> {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<WeaponNameText>>();
    let mut results: Vec<Entity> = query.iter(app.world()).collect();
    if results.len() == 1 {
        results.pop()
    } else {
        None
    }
}

#[test]
fn longest_shipped_weapon_name_does_not_overlap_reload_button() {
    let longest_name = longest_shipped_ranged_weapon_name();
    assert!(
        !longest_name.is_empty(),
        "must find at least one shipped ranged weapon .ron under assets/content/weapons/ranged/",
    );

    let app_opt = real_layout_battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real-layout harness must reach BattleScapeState::BattleRunning with its font loaded",
    );
    let Some(mut app) = app_opt else { return };

    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            &longest_name,
            Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18)),
        ),
    );
    for _ in 0..8 {
        app.update();
    }

    let name_rect = pixel_rect_of::<WeaponNameText>(&mut app);
    let reload_rect = pixel_rect_of::<ReloadButton>(&mut app);
    assert!(
        name_rect.is_some() && reload_rect.is_some(),
        "both the weapon name label and the Reload button must exist with real computed geometry",
    );
    let (Some(name_rect), Some(reload_rect)) = (name_rect, reload_rect) else {
        return;
    };
    assert!(
        !name_rect.overlaps(&reload_rect),
        "the weapon name label ({:?}..{:?}) must NOT overlap the Reload button ({:?}..{:?}) — \
         the name row and the Reload button must not share the same pixels",
        name_rect.min,
        name_rect.max,
        reload_rect.min,
        reload_rect.max,
    );

    let content_rect = pixel_rect_of::<WeaponContent>(&mut app);
    let name_entity = weapon_name_text_entity(&mut app);
    assert!(
        content_rect.is_some() && name_entity.is_some(),
        "the weapon-text block and the name entity must exist",
    );
    let (Some(content_rect), Some(name_entity)) = (content_rect, name_entity) else {
        return;
    };
    let layout_info = app.world().get::<TextLayoutInfo>(name_entity);
    assert!(
        layout_info.is_some(),
        "the name text must have a real measured TextLayoutInfo (the real-layout harness's font \
         must have finished loading)",
    );
    let Some(layout_info) = layout_info else {
        return;
    };
    let content_width = content_rect.max.x - content_rect.min.x;
    assert!(
        layout_info.size.x <= content_width + EPSILON_PX,
        "the weapon name \"{longest_name}\"'s REAL measured glyph width ({}) must fit inside its \
         text block's width ({content_width}) — it must not silently overflow past its own box",
        layout_info.size.x,
    );
}
