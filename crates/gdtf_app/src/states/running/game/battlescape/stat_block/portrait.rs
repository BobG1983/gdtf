//! Stat-block portrait index and spawn helpers.

use std::hash::{Hash, Hasher};

use bevy::{
    ecs::template::template,
    image::TextureAtlas,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::{Node, Val, widget::ImageNode},
};
use gdtf_battle_presenter::{SheetRole, TopDownAtlases};
use gdtf_battle_sim::ganger::GangerName;

use crate::states::running::game::battlescape::stat_block::components::StatPortrait;

const PORTRAIT_COUNT: usize = 100;

const PORTRAIT_VH: f32 = 8.0;

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::game::battlescape) struct PortraitIndex(usize);

impl PortraitIndex {
    #[must_use]
    pub(in crate::states::running::game::battlescape) fn for_name(
        name: Option<&GangerName>,
    ) -> Self {
        let Some(name) = name else {
            return Self(0);
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (**name).hash(&mut hasher);
        Self(usize::try_from(hasher.finish() % PORTRAIT_COUNT as u64).unwrap_or(0))
    }
}

/// Portrait atlas index for a ganger name (test helper).
#[cfg(feature = "headless_test")]
#[must_use]
pub fn portrait_index_for_name(name: Option<&GangerName>) -> usize {
    *PortraitIndex::for_name(name)
}

pub(in crate::states::running::game::battlescape) fn portrait_node(
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    index: PortraitIndex,
) -> (ImageNode, Node) {
    (
        ImageNode::from_atlas_image(
            image,
            TextureAtlas {
                layout,
                index: *index,
            },
        ),
        Node {
            width: Val::Vh(PORTRAIT_VH),
            height: Val::Vh(PORTRAIT_VH),
            ..default()
        },
    )
}

pub(super) fn spawn_portrait(commands: &mut Commands, atlases: Option<&TopDownAtlases>) -> Entity {
    if let Some(sheet) = atlases.and_then(|a| a.role(SheetRole::Portraits)) {
        let (image_node, node) = portrait_node(
            sheet.image.clone(),
            sheet.layout.clone(),
            PortraitIndex::for_name(None),
        );
        commands
            .spawn_scene((
                bsn! {
                    StatPortrait
                    template(move |_| Ok(image_node.clone()))
                },
                template_value(node),
            ))
            .id()
    } else {
        let node = Node {
            width: Val::ZERO,
            height: Val::ZERO,
            ..default()
        };
        commands
            .spawn_scene((
                bsn! {
                    StatPortrait
                    template(|_| Ok(ImageNode::default()))
                },
                template_value(node),
            ))
            .id()
    }
}
