//! Floating combat text spawn and rise animation.

use std::time::Duration;

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    sprite::Anchor,
    text::{FontSize, FontWeight},
};
use gdtf_battle_sim::prelude::CellLevel;

use super::{
    super::tuning::{FctRiseRate, FctTtlSeconds, FxTuning},
    slot_allocator::FctAnchorCell,
};
use crate::{Layer, cell_to_world_layered};

/// Display string for a floating combat pop.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct CombatText(String);

impl CombatText {
    /// Build from any string-like value.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// Vertical stack slot when multiple pops share a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref)]
pub struct FctStackIndex(usize);

impl FctStackIndex {
    /// Bottom-most slot.
    pub const BASE: Self = Self(0);

    /// Build from a zero-based stack index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

const STACK_STEP_PX: f32 = 12.0;

const FCT_FONT_PT: f32 = 14.0;

const FCT_BOLD_FONT_SCALE: f32 = 1.4;

/// Font weight emphasis for a pop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FctEmphasis {
    /// Regular weight.
    #[default]
    Normal,
    /// Bold weight (lethal / critical).
    Bold,
}

impl FctEmphasis {
    /// Bevy font weight for this emphasis.
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

/// What a floating combat pop says and how it reads.
#[derive(Debug, Clone, PartialEq)]
pub struct FctLabel {
    text:     CombatText,
    color:    Color,
    emphasis: FctEmphasis,
}

impl FctLabel {
    /// Build a pop's text, tint, and weight.
    #[must_use]
    pub const fn new(text: CombatText, color: Color, emphasis: FctEmphasis) -> Self {
        Self {
            text,
            color,
            emphasis,
        }
    }
}

/// Where a pop sits: its cell and its place in that cell's stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FctSlot {
    at:    CellLevel,
    stack: FctStackIndex,
}

impl FctSlot {
    /// Build from a cell and the stack slot allocated for it.
    #[must_use]
    pub const fn new(at: CellLevel, stack: FctStackIndex) -> Self {
        Self { at, stack }
    }
}

/// How a pop rises and fades.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FctDrift {
    rise: FctRiseRate,
    ttl:  FctTtlSeconds,
}

impl FctDrift {
    /// Build from a rise rate and a lifetime.
    #[must_use]
    pub const fn new(rise: FctRiseRate, ttl: FctTtlSeconds) -> Self {
        Self { rise, ttl }
    }

    /// Read the shipped rise rate and lifetime from FX tuning.
    #[must_use]
    pub const fn from_tuning(tuning: &FxTuning) -> Self {
        Self::new(tuning.fct_rise_rate, tuning.fct_ttl_seconds)
    }
}

/// Component driving rise and fade of a floating combat text entity.
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

/// Spawn a floating combat text pop in its allocated stack slot.
pub fn spawn_floating_text(
    commands: &mut Commands,
    label: FctLabel,
    slot: FctSlot,
    drift: FctDrift,
) {
    let (cell, level) = slot.at.split();
    let mut world = cell_to_world_layered(cell, level, Layer::Highlight);
    world.y = (*slot.stack as f32).mul_add(-STACK_STEP_PX, world.y);

    let base_alpha = label.color.alpha();

    let text_2d = Text2d::new((*label.text).clone());
    let text_font = TextFont {
        font_size: FontSize::Px(label.emphasis.font_size()),
        weight: label.emphasis.weight(),
        ..default()
    };
    let transform = Transform::from_translation(world);
    let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
    let pop = FloatingCombatText::new(drift.rise, drift.ttl, base_alpha);

    commands
        .spawn_scene((
            bsn! { template(move |_| Ok(text_2d.clone())) },
            bsn! { template(move |_| Ok(text_font.clone())) },
            template_value(TextColor(label.color)),
            template_value(Anchor::BOTTOM_CENTER),
            template_value(transform),
            template_value(layers),
            bsn! { template(move |_| Ok(pop.clone())) },
        ))
        .insert(FctAnchorCell::new(slot.at));
}

/// Rise and fade floating combat text; despawn when expired.
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
