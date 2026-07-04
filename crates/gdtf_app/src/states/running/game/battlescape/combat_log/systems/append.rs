//! The ONE combat-log APPENDER (GTW-572 C5) — drains the buffered
//! [`CombatLogEvent`](gdtf_battle_presenter::CombatLogEvent)s the per-source forwarders
//! wrote, classifies each through the shared
//! [`classify_log_event`](gdtf_battle_presenter::classify_log_event), spawns one UI text
//! line per [`LogLine`](gdtf_battle_presenter::LogLine) under [`CombatLogRoot`], and
//! FIFO-trims to the tuned cap with the GTW-328 slide semantics.
//!
//! It preserves the pre-GTW-572 semantics exactly: the single-pass FIFO trim (oldest lines
//! despawn, survivors SLIDE into the freed slots), and the drain-don't-replay rootless
//! behavior (no container / theme yet → the buffered events are DRAINED and dropped, never
//! replayed once the log exists).

use bevy::{
    color::Alpha,
    prelude::*,
    scene::{CommandsSceneExt, bsn},
    text::{FontSize, FontWeight, LineHeight, TextFont},
    ui::{ComputedNode, Node, PositionType, Val},
};
use gdtf_battle_presenter::{CombatLogEvent, FctEmphasis, LogLine, classify_log_event};
use gdtf_ui::theme::GdtfTheme;

use crate::states::running::game::battlescape::combat_log::{
    components::{CombatLogLine, CombatLogRoot, LineSlide, LogLineFade},
    tuning::CombatLogTuning,
};

/// Drain the buffered [`CombatLogEvent`]s, classify each into log lines, and append them
/// under the combat-log root, FIFO-trimming to the tuned visible cap (GTW-572 C5).
///
/// Runs in `Update` (`CombatLogSystems::Append`, strictly AFTER every forwarder — the
/// explicit set chain), gated on the live-battle witness. For each drained event it
/// classifies with the shared [`classify_log_event`] and spawns one UI text line per
/// [`LogLine`] under [`CombatLogRoot`], then despawns the OLDEST lines until the visible
/// count is within [`max_visible_lines`](CombatLogTuning::max_visible_lines).
///
/// No-ops cleanly if the log root / theme are absent (`bevy-traps.md` #1): the buffered
/// events are DRAINED so a pre-spawn event is not replayed once the log exists (the
/// pre-GTW-572 `.clear()` semantics). The tuning is DEFAULTED if its async RON has not
/// resolved yet. Param-only (`bevy-traps.md` #7): [`Commands`], the event reader, the root
/// + line queries, and the theme + tuning reads.
pub(in crate::states::running::game::battlescape) fn append_combat_log(
    mut commands: Commands,
    mut events: MessageReader<CombatLogEvent>,
    root: Query<Entity, With<CombatLogRoot>>,
    mut lines: Query<(Entity, &ComputedNode, &mut LineSlide), With<CombatLogLine>>,
    theme: Option<Res<GdtfTheme>>,
    tuning: Option<Res<CombatLogTuning>>,
) {
    let (Ok(root), Some(theme)) = (root.single(), theme) else {
        // No log container / theme yet — DRAIN the buffered events so a pre-spawn event is
        // not replayed once the log exists, then bail (the lines would have nowhere to go).
        events.clear();
        return;
    };
    // The tuning's RON resolve is async; default it (TTL / fade / cap) if it has not
    // resolved yet so a combat event still logs at the sane defaults.
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);

    // Classify each resolved event into its lines and append a UI text node per line under
    // the root, collecting the freshly-spawned line ids (in append / chronological order) so
    // the trim can include them — the deferred new lines are NOT yet in the `lines` query.
    let mut appended: Vec<Entity> = Vec::new();
    for event in events.read() {
        for line in classify_log_event(event) {
            let entity = spawn_log_line(&mut commands, &theme, &tuning, &line);
            commands.entity(root).add_children(&[entity]);
            appended.push(entity);
        }
    }

    trim_to_cap(
        &mut commands,
        &mut lines,
        *tuning.max_visible_lines,
        &appended,
    );
}

/// Despawn the OLDEST log lines so the total visible count is within `cap`, and SLIDE the
/// survivors into the freed slots rather than snapping (GTW-328 slice B).
///
/// The full chronological line set is the prior-frame `existing` lines (sorted ascending by
/// [`Entity`], which for a monotonic spawner is spawn order — oldest first) FOLLOWED BY this
/// frame's `appended` lines in append order (the deferred new lines are not yet in the
/// `existing` query, so they are threaded in explicitly). If that total exceeds `cap`, the
/// first `overflow` of that combined order — the oldest — are despawned (FIFO). A `cap` of
/// `0` is treated as unbounded (despawn nothing) so a malformed tuning value cannot blank
/// the log.
///
/// On a removal, each removed line's logical height (its [`ComputedNode`] size scaled by
/// the inverse scale factor) is summed and that freed height is ADDED to every SURVIVING
/// prior line's [`LineSlide`] offset, so the survivors hold their old screen position this
/// frame and then ease UP into the gap (`slide_combat_log_lines`).
fn trim_to_cap(
    commands: &mut Commands,
    lines: &mut Query<(Entity, &ComputedNode, &mut LineSlide), With<CombatLogLine>>,
    cap: usize,
    appended: &[Entity],
) {
    if cap == 0 {
        return;
    }
    // Oldest-first: prior lines (spawn-ordered) then this frame's appended lines.
    let mut chronological: Vec<Entity> = lines.iter().map(|(e, ..)| e).collect();
    chronological.sort_unstable();
    chronological.extend_from_slice(appended);

    let overflow = chronological.len().saturating_sub(cap);
    if overflow == 0 {
        return;
    }
    let removed: Vec<Entity> = chronological.into_iter().take(overflow).collect();

    // Sum the freed logical height of the removed lines so the survivors slide into the gap.
    let mut freed_height = 0.0_f32;
    for &entity in &removed {
        if let Ok((_, node, _)) = lines.get(entity) {
            freed_height = node
                .size()
                .y
                .mul_add(node.inverse_scale_factor(), freed_height);
        }
    }

    // Bump every surviving line's slide by the freed height (it holds its old screen
    // position then eases up); skip the lines being removed.
    if freed_height > f32::EPSILON {
        for (entity, _, mut slide) in lines.iter_mut() {
            if !removed.contains(&entity) {
                slide.displace(freed_height);
            }
        }
    }

    for entity in removed {
        commands.entity(entity).despawn();
    }
}

/// The line-height multiple of a line's font size used to SEED its [`LineSlide`]
/// appear-offset — a fresh line is displaced ~one line-height below its slot, then slides
/// UP into place (GTW-328 slice B). A framework-plumbing layout `const` (the carve-out):
/// an estimate of the rendered line height before the layout engine computes the real
/// [`ComputedNode`], so the exact value is cosmetic (the slide eases to `0` regardless).
const APPEAR_OFFSET_LINE_HEIGHTS: f32 = 1.3;

/// The line-height multiple of a combat-log line's font size — the full glyph box reserved
/// per line (ascenders + descenders + leading), set EXPLICITLY as [`LineHeight::Px`] and
/// matched by the line node's `min_height` so the clip never shaves a settled line (the
/// cut-off fix).
///
/// A framework-plumbing layout `const` (the `ui-responsive-not-px` carve-out — a
/// glyph-relative multiple, not a fixed px). Bevy's [`LineHeight`] default is
/// `RelativeToFont(1.2)`; this is a touch more generous (`1.4`) so descenders + leading
/// have headroom and a resting line is fully legible rather than clipped flush at the
/// panel's bottom edge.
const LINE_HEIGHT_SCALE: f32 = 1.4;

/// Spawn ONE combat-log line as a UI text node carrying the classified [`LogLine`]'s text +
/// color + emphasis-weight, plus the [`CombatLogLine`] marker, its [`LogLineFade`] clock,
/// and its [`LineSlide`] appear-offset (GTW-328 slice B).
///
/// Post-BSN spawn idiom (`bsn!` / `spawn_scene`): `Text` + `UiTextColor` ride the `bsn!`
/// macro inline; the runtime-valued `TextFont` (not `Unpin`) rides the
/// `template(move |_| ..)` closure escape hatch (the weapon-panel `spawn_text` precedent).
/// The line draws at the tuned [`line_font_pt`](CombatLogTuning::line_font_pt) — the log's
/// OWN readable body size — and a lethal DOWN / DEAD / dies line maps to
/// [`FontWeight::BOLD`] + a size bump ([`emphasis_style`]). The line spawns at `alpha 0`
/// (its fade-in ramps it on). Returns the line entity.
fn spawn_log_line(
    commands: &mut Commands,
    theme: &GdtfTheme,
    tuning: &CombatLogTuning,
    line: &LogLine,
) -> Entity {
    let text = (**line.text()).clone();
    let mut color = line.color();
    let base_alpha = color.alpha();
    let (weight, font_size) = emphasis_style(line.emphasis(), *tuning.line_font_pt);
    let font = theme.text.font.clone();
    let text_font = TextFont {
        font: font.into(),
        font_size: FontSize::Px(font_size),
        weight,
        ..default()
    };
    // Reserve the FULL glyph box (ascenders + descenders + leading) so the clip never
    // shaves a settled line — set explicitly rather than relying on bevy's
    // RelativeToFont(1.2) default.
    let line_box_px = font_size * LINE_HEIGHT_SCALE;
    let fade = LogLineFade::new(
        tuning.line_ttl_seconds,
        tuning.fade_in_seconds,
        tuning.fade_out_seconds,
        base_alpha,
    );
    // Seed the appear-offset ~one line-height below the slot so the line slides UP into place.
    let slide = LineSlide::new(font_size * APPEAR_OFFSET_LINE_HEIGHTS);
    // Spawn TRANSPARENT — the fade-in ramps the alpha on from 0 (no full-opacity flash frame).
    color.set_alpha(0.0);
    commands
        .spawn_scene(bsn! {
            Text::new(text)
            TextColor(color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert((
            CombatLogLine,
            fade,
            slide,
            // The full glyph box per line (matches `min_height` below) so a settled line is
            // never shaved by the panel clip — descenders + leading have headroom.
            LineHeight::Px(line_box_px),
            // A RELATIVE node whose `top` carries the slide offset (the line keeps its
            // flex-column slot; `top` displaces it from that slot, eased to 0 by
            // `slide_combat_log_lines`). Its `min_height` reserves the full line box so the
            // laid-out line never collapses shorter than its glyphs.
            Node {
                position_type: PositionType::Relative,
                min_height: Val::Px(line_box_px),
                top: Val::Px(font_size * APPEAR_OFFSET_LINE_HEIGHTS),
                ..default()
            },
        ))
        .id()
}

/// The (weight, size) a classified line's [`FctEmphasis`] draws at — BOLD at a size bump
/// for a lethal line, the base weight/size otherwise.
///
/// Mirrors the presenter's private `FctEmphasis::weight` / `font_size` mapping (which is
/// not `pub`), so the log line reads heavier for the lethal line on the bundled
/// non-variable font.
fn emphasis_style(emphasis: FctEmphasis, base_pt: f32) -> (FontWeight, f32) {
    match emphasis {
        FctEmphasis::Bold => (FontWeight::BOLD, base_pt * BOLD_FONT_SCALE),
        FctEmphasis::Normal => (FontWeight::NORMAL, base_pt),
    }
}

/// The size multiplier a BOLD (lethal) combat-log line is drawn at, on top of the theme's
/// body size — the visible heavier-weight lever on the bundled non-variable font (the FCT
/// precedent).
const BOLD_FONT_SCALE: f32 = 1.25;
