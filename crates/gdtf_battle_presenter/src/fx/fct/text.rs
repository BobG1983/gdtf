//! The reusable FLOATING-COMBAT-TEXT primitive — the world-anchored rise/fade pop and the
//! one system that animates it.
//!
//! A floating-combat-text (FCT) pop is a short [`Text2d`] number / tag that appears over a
//! battlefield cell, RISES a little as it FADES to transparent, then despawns — the genre
//! convention for "this much damage, here". This module is the GENERIC primitive ONLY: the
//! [`spawn_floating_text`] helper (caller picks the string, color, cell / level, and a
//! stack index) and the [`animate_floating_text`] system that drives every live pop's rise,
//! fade, and TTL despawn. It reads NO sim message — the reader slices (3-4) classify the
//! [`ShotFired`](gdtf_battle_sim::ShotFired) consequences and CALL [`spawn_floating_text`]
//! with a color from the [`palette`](super::palette).
//!
//! Lifecycle mirrors the transient FX flash ([`FlashTtl`](super::super::FlashTtl) /
//! `expire_flashes`): the pop is spawned ONCE carrying its own [`FloatingCombatText`]
//! state, and [`animate_floating_text`] MUTATES that one entity each frame (translation up,
//! alpha down) — never respawned per frame — until its [`FctTtlSeconds`] clock finishes,
//! at which point it is despawned. Simultaneous pops on one cell are vertically
//! STACK-OFFSET by their `stack_index` so they do not overlap.

use std::time::Duration;

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    sprite::Anchor,
    text::{FontSize, FontWeight},
};
use gdtf_battle_sim::{Cell, Level};

use super::super::tuning::{FctRiseRate, FctTtlSeconds};
use crate::{Layer, cell_to_world_layered};

/// The combat-text STRING a pop renders — a damage number, a wound tag, a "miss".
///
/// A NAMED newtype over the displayed [`String`] (no-bare-types: the rendered combat text
/// is a domain value, not a bare `String`), [`Deref`]ing to `str` so a reader inspects it
/// straight through. The reader slices build it (e.g. `format!("-{hp}")`); the primitive
/// only renders it.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct CombatText(String);

impl CombatText {
    /// Build a combat-text string from anything string-like (a damage number, a tag).
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// The per-cell vertical STACK SLOT of a floating-combat-text pop.
///
/// A NAMED newtype over the stack index (no-bare-types: a stacking slot is a domain value,
/// not a bare `usize`), [`Deref`]ing to it. Several pops landing on ONE cell the same tick
/// would overlap; the caller hands each successive pop the next index (0, 1, 2, …) and
/// [`spawn_floating_text`] offsets its spawn `y` DOWN by `index × STACK_STEP_PX` so they
/// fan out vertically and stay legible. Index `0` is the unshifted base slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref)]
pub struct FctStackIndex(usize);

impl FctStackIndex {
    /// The base (unshifted) stack slot — the first pop on a cell this tick.
    pub const BASE: Self = Self(0);

    /// Build a stack slot from its index (0 = the base slot, 1 = one step down, …).
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// The world-px vertical step between successive stacked pops on one cell.
///
/// A `const`, NOT a domain newtype — the framework-plumbing carve-out
/// (`.claude/rules/no-bare-types.md` clause 4): a layout offset fed straight into a
/// [`Transform`] translation, the same reasoning the landed `CELL_PX`-class consts use.
/// Sized just under one cell so two simultaneous pops on a cell read as a small stack
/// rather than colliding.
const STACK_STEP_PX: f32 = 12.0;

/// The font size of a floating-combat-text pop, in points.
///
/// A `const` font size in `pt` — the documented `font_size_pt` carve-out
/// (`.claude/rules/ui-responsive-not-px.md`: relative units everywhere EXCEPT font size,
/// which is `pt`). Sized to read clearly over a 16px world tile without dwarfing it.
const FCT_FONT_PT: f32 = 14.0;

/// The font-size multiplier an [`FctEmphasis::Bold`] pop is drawn at, on top of
/// [`FCT_FONT_PT`].
///
/// A `const` scale factor — framework plumbing (no domain meaning beyond "draw this pop
/// bigger"). The default [`Font`] is a NON-variable face, on which [`FontWeight::BOLD`]
/// alone may not visibly thicken the strokes; pairing the weight with this size bump is the
/// reliable, asset-free emphasis lever, so a bold pop reads heavier even on the bundled font.
const FCT_BOLD_FONT_SCALE: f32 = 1.4;

/// How a floating-combat-text pop is WEIGHTED — its emphasis tier on top of the color.
///
/// A named domain enum (no bare `bool` / weight number): a pop is either [`Normal`](FctEmphasis::Normal)
/// (the default body weight) or [`Bold`](FctEmphasis::Bold) (the heaviest, most-urgent pop —
/// a lethal DOWN / DEAD). [`spawn_floating_text`] turns this into the pop's [`TextFont`]
/// `weight` + `font_size`: a [`Bold`](FctEmphasis::Bold) pop is drawn in [`FontWeight::BOLD`]
/// AND scaled up by [`FCT_BOLD_FONT_SCALE`] (the size bump is the visible lever on the bundled
/// non-variable font, the weight is correct on any variable font). This is the styling
/// attribute the GTW-302 contract's "DOWN / DEAD (RED bold)" clause requires — distinct from
/// the color, which the caller passes separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FctEmphasis {
    /// The default body weight — every ordinary damage / wound / miss / status pop.
    #[default]
    Normal,
    /// The heaviest pop — a lethal DOWN / DEAD. Drawn [`FontWeight::BOLD`] at
    /// [`FCT_BOLD_FONT_SCALE`]× the base size.
    Bold,
}

impl FctEmphasis {
    /// The [`FontWeight`] this emphasis tier draws in — [`FontWeight::BOLD`] for
    /// [`Bold`](FctEmphasis::Bold), [`FontWeight::NORMAL`] otherwise.
    #[must_use]
    const fn weight(self) -> FontWeight {
        match self {
            Self::Normal => FontWeight::NORMAL,
            Self::Bold => FontWeight::BOLD,
        }
    }

    /// The base [`FCT_FONT_PT`] size scaled for this emphasis tier — bumped by
    /// [`FCT_BOLD_FONT_SCALE`] for [`Bold`](FctEmphasis::Bold), the base size otherwise.
    #[must_use]
    fn font_size(self) -> f32 {
        match self {
            Self::Normal => FCT_FONT_PT,
            Self::Bold => FCT_FONT_PT * FCT_BOLD_FONT_SCALE,
        }
    }
}

/// One live floating-combat-text pop's animation state — its rise rate, its remaining-life
/// clock, and its base alpha.
///
/// A NAMED grouping component (not a bare tuple): [`animate_floating_text`] advances the
/// `ttl` clock each frame, rises the [`Transform`] by `rise × delta`, and fades the
/// [`TextColor`] alpha from `base_alpha` toward `0` across the pop's lifetime, despawning
/// it when the clock finishes. The clock + the elapsed fraction mutate ONLY through
/// [`advance`](FloatingCombatText::advance) (no `DerefMut`). `base_alpha` is captured at
/// spawn so the fade is a fraction of the pop's STARTING opacity (a caller may spawn a
/// translucent pop).
#[derive(Component, Debug, Clone)]
pub struct FloatingCombatText {
    /// The ascent speed (world px/sec) the pop's `y` grows by each frame.
    rise:       FctRiseRate,
    /// The one-shot remaining-life clock; the pop despawns the frame it finishes.
    ttl:        Timer,
    /// The pop's STARTING alpha, captured at spawn so the fade scales from it toward 0.
    base_alpha: f32,
}

impl FloatingCombatText {
    /// Start a fresh pop with ascent `rise`, lifetime `ttl`, and starting opacity
    /// `base_alpha`.
    ///
    /// The lifetime is a [`TimerMode::Once`] clock (it finishes exactly once — the despawn
    /// signal). `base_alpha` is the spawn color's alpha so a translucent pop fades from its
    /// own opacity rather than full.
    #[must_use]
    fn new(rise: FctRiseRate, ttl: FctTtlSeconds, base_alpha: f32) -> Self {
        Self {
            rise,
            ttl: Timer::from_seconds(*ttl, TimerMode::Once),
            base_alpha,
        }
    }

    /// Advance the pop by `delta` and report whether its lifetime has now elapsed.
    ///
    /// Ticks the one-shot clock; returns `true` once the lifetime window has finished — the
    /// signal [`animate_floating_text`] despawns the pop on. Wraps the inner [`Timer`] so it
    /// mutates through a named method (no `DerefMut`).
    fn advance(&mut self, delta: Duration) -> bool {
        self.ttl.tick(delta).is_finished()
    }

    /// The world-px the pop has RISEN so far — its ascent speed × elapsed life.
    ///
    /// The pop's `y` is its spawn `y` plus this; growing it each frame is the rise.
    #[must_use]
    fn risen(&self) -> f32 {
        self.rise.mul_add(self.ttl.elapsed_secs(), 0.0)
    }

    /// The pop's CURRENT alpha — its `base_alpha` scaled by the remaining-life fraction, so
    /// it fades linearly from full opacity at spawn to transparent at expiry.
    ///
    /// `fraction_remaining` is `1.0` at spawn and `0.0` at expiry (Bevy's
    /// [`Timer::fraction_remaining`]), so the alpha is `base_alpha` at spawn and `0` as the
    /// clock finishes — the fade.
    #[must_use]
    fn alpha(&self) -> f32 {
        self.base_alpha * self.ttl.fraction_remaining()
    }
}

/// Spawn ONE floating-combat-text pop showing `text` in `color` at `emphasis`, anchored over
/// the `(cell, level)` world point, at vertical stack slot `stack_index`.
///
/// The pop is a world-space [`Text2d`] anchored at its BOTTOM-CENTER over the cell
/// ([`cell_to_world_layered`] at the [`Layer::Highlight`] band so it draws ABOVE the
/// ganger / terrain / FX sprites at that cell), shifted DOWN by `*stack_index × STACK_STEP_PX`
/// so simultaneous pops on the cell fan out instead of overlapping. It carries a fresh
/// [`FloatingCombatText`] (the passed-in [`FctRiseRate`] ascent + [`FctTtlSeconds`] lifetime,
/// capturing `color`'s alpha as the fade base) and the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER) so it renders on the world camera.
/// [`animate_floating_text`] then drives its rise + fade + despawn.
///
/// `ttl` / `rise` are the pop's lifetime + ascent speed, passed in by the caller from the
/// hot-reloadable [`FxTuning`](super::super::FxTuning) (GTW-327) rather than read from a
/// `const`, so a live `fx_tuning.ron` edit re-tunes the very next pop.
///
/// `emphasis` is the styling WEIGHT ([`FctEmphasis`]): a [`Bold`](FctEmphasis::Bold) pop (the
/// lethal DOWN / DEAD) is drawn in [`FontWeight::BOLD`] at a larger [`FctEmphasis::font_size`],
/// so the contract's "RED bold" lethal tag reads as the heaviest pop; an ordinary pop is
/// [`Normal`](FctEmphasis::Normal). The weight + size are applied to the pop's one [`TextFont`].
///
/// `cell` / `level` are the typed sim grid position (the world anchor); `text` / `color` /
/// `emphasis` / `stack_index` are the pop's content + color + weight + stacking slot. Pure
/// VIEW: it only spawns a presenter entity (ADR-0001), reads + writes no sim state.
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
    // Anchor over the cell on the highlight band (above gangers/terrain/FX), then shift the
    // whole pop DOWN by its stack slot so simultaneous pops on one cell fan out vertically.
    let mut world = cell_to_world_layered(cell, level, Layer::Highlight);
    world.y = (*stack_index as f32).mul_add(-STACK_STEP_PX, world.y);

    let base_alpha = color.alpha();

    // GTW-322 — authored as a `bsn!` scene (the SAME entity tree the old spawn tuple produced,
    // only the spawn SHAPE changed). `Text2d` and `TextFont` are NOT `Unpin` (the same family as
    // the AREA-1 widget builders' text), and the `FloatingCombatText` animation state (a `Timer`
    // + the rise / base-alpha) has no `Default`, so all three ride the
    // `template(move |_| Ok(value.clone()))` closure escape hatch (the `FnTemplate` output is
    // bound by neither `Unpin` nor `Default`). `TextColor`, `Anchor`, `Transform`, and
    // `RenderLayers` are all `Clone + Default + Unpin`, so each rides `template_value` (a
    // value-overwrite). The pop materializes on this frame's `SpawnScene` schedule; the
    // unguarded `animate_floating_text` picks it up to rise / fade / despawn it.
    let text_2d = Text2d::new((*text).clone());
    let text_font = TextFont {
        // The emphasis tier picks BOTH the weight (correct on a variable font) and the
        // size bump (the visible lever on the bundled non-variable font), so a Bold pop
        // reads heavier regardless of which font is loaded.
        font_size: FontSize::Px(emphasis.font_size()),
        weight: emphasis.weight(),
        ..default()
    };
    let transform = Transform::from_translation(world);
    let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
    let pop = FloatingCombatText::new(rise, ttl, base_alpha);

    commands.spawn_scene((
        bsn! { template(move |_| Ok(text_2d.clone())) },
        bsn! { template(move |_| Ok(text_font.clone())) },
        template_value(TextColor(color)),
        // Bottom-center so the pop sits ON the cell and rises up off it.
        template_value(Anchor::BOTTOM_CENTER),
        template_value(transform),
        template_value(layers),
        bsn! { template(move |_| Ok(pop.clone())) },
    ));
}

/// `Update` (`PresenterSystems::Draw`): rise + fade + despawn every live floating-combat-text
/// pop.
///
/// Advances each [`FloatingCombatText`] by the frame [`Res<Time>`] delta. While the pop is
/// alive it MUTATES that one entity in place (no respawn): the `y` of its [`Transform`]
/// becomes its spawn `y` plus the risen distance ([`FloatingCombatText::risen`]), and its
/// [`TextColor`] alpha is set to the remaining-life fraction of its base alpha
/// ([`FloatingCombatText::alpha`]) — the rise + the fade. The frame the pop's
/// [`FctTtlSeconds`] lifetime finishes ([`FloatingCombatText::advance`] returns `true`), it
/// is `Commands::entity(e).despawn`ed. It touches ONLY [`FloatingCombatText`]-marked
/// entities; it needs no `BattleInProgress` gate (inert with no pops — the query is empty —
/// so a pop spawned during a battle still completes after it ends), mirroring `expire_flashes`.
///
/// The pop's spawn `y` is recovered as `translation.y − risen` so the rise is monotonic and
/// frame-rate independent (it re-derives the base each frame from the elapsed-life ascent
/// rather than accumulating per-frame deltas, which would drift). Param-only
/// (`bevy-traps.md` #7): [`Commands`] for the despawn, [`Res<Time>`] for the delta, and the
/// `(Entity, &mut Transform, &mut TextColor, &mut FloatingCombatText)` query.
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
        // Recover the spawn baseline BEFORE this frame's tick advances the ascent, so the
        // rise re-derives from elapsed life (monotonic, frame-rate independent) rather than
        // accumulating per-frame deltas.
        let base_y = transform.translation.y - pop.risen();
        let expired = pop.advance(delta);
        if expired {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.y = base_y + pop.risen();
        // Fade the alpha toward 0 across the lifetime (mutate the existing color in place).
        color.0.set_alpha(pop.alpha());
    }
}
