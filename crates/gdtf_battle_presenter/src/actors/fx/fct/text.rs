use std::time::Duration;

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    sprite::Anchor,
    text::{FontSize, FontWeight},
};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

use super::{
    super::tuning::{FctRiseRate, FctTtlSeconds},
    slot_allocator::FctAnchorCell,
};
use crate::{Layer, cell_to_world_layered};

#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct CombatText(String);

impl CombatText {
        #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref)]
pub struct FctStackIndex(usize);

impl FctStackIndex {
        pub const BASE: Self = Self(0);

        #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

const STACK_STEP_PX: f32 = 12.0;

const FCT_FONT_PT: f32 = 14.0;

const FCT_BOLD_FONT_SCALE: f32 = 1.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FctEmphasis {
        #[default]
    Normal,
            Bold,
}

impl FctEmphasis {
                                    #[must_use]
    pub const fn weight(self) -> FontWeight {
        match self {
            Self::Normal => FontWeight::NORMAL,
            Self::Bold => FontWeight::BOLD,
        }
    }

            #[must_use]
    fn font_size(self) -> f32 {
        match self {
            Self::Normal => FCT_FONT_PT,
            Self::Bold => FCT_FONT_PT * FCT_BOLD_FONT_SCALE,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct FloatingCombatText {
        rise:       FctRiseRate,
        ttl:        Timer,
        base_alpha: f32,
}

impl FloatingCombatText {
                            #[must_use]
    fn new(rise: FctRiseRate, ttl: FctTtlSeconds, base_alpha: f32) -> Self {
        Self {
            rise,
            ttl: Timer::from_seconds(*ttl, TimerMode::Once),
            base_alpha,
        }
    }

                        fn advance(&mut self, delta: Duration) -> bool {
        self.ttl.tick(delta).is_finished()
    }

                #[must_use]
    fn risen(&self) -> f32 {
        self.rise.mul_add(self.ttl.elapsed_secs(), 0.0)
    }

                            #[must_use]
    fn alpha(&self) -> f32 {
        self.base_alpha * self.ttl.fraction_remaining()
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the FCT pop's content, color, weight, world anchor, stack slot, and now its \
              hot-reloadable lifetime + rise are all distinct caller-chosen inputs"
)]
pub fn spawn_floating_text(
    commands: &mut Commands,
    text: CombatText,
    color: Color,
    emphasis: FctEmphasis,
    cell: Cell,
    level: Level,
    stack_index: FctStackIndex,
    ttl: FctTtlSeconds,
    rise: FctRiseRate,
) {
    let mut world = cell_to_world_layered(cell, level, Layer::Highlight);
    world.y = (*stack_index as f32).mul_add(-STACK_STEP_PX, world.y);

    let base_alpha = color.alpha();

    let text_2d = Text2d::new((*text).clone());
    let text_font = TextFont {
        font_size: FontSize::Px(emphasis.font_size()),
        weight: emphasis.weight(),
        ..default()
    };
    let transform = Transform::from_translation(world);
    let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
    let pop = FloatingCombatText::new(rise, ttl, base_alpha);

    commands
        .spawn_scene((
            bsn! { template(move |_| Ok(text_2d.clone())) },
            bsn! { template(move |_| Ok(text_font.clone())) },
            template_value(TextColor(color)),
            template_value(Anchor::BOTTOM_CENTER),
            template_value(transform),
            template_value(layers),
            bsn! { template(move |_| Ok(pop.clone())) },
        ))
        .insert(FctAnchorCell::new(CellLevel::new(cell, level)));
}

pub fn animate_floating_text(
    mut commands: Commands,
    time: Res<Time>,
    mut pops: Query<(
        Entity,
        &mut Transform,
        &mut TextColor,
        &mut FloatingCombatText,
    )>,
) {
    let delta = time.delta();
    for (entity, mut transform, mut color, mut pop) in &mut pops {
        let base_y = transform.translation.y - pop.risen();
        let expired = pop.advance(delta);
        if expired {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.y = base_y + pop.risen();
        color.0.set_alpha(pop.alpha());
    }
}
