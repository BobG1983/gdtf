//! Drains the combat-event messages into combat-log lines, and fades + FIFO-trims them
//! (GTW-328, slice 3).
//!
//! [`update_combat_log`] runs in `Update` (gated on the live-battle witness): it drains the
//! event-driven sim combat-event messages ([`FireDeclaration`](gdtf_battle_sim::FireDeclaration),
//! [`MovementOccurred`](gdtf_battle_sim::MovementOccurred),
//! [`TurnStarted`](gdtf_battle_sim::TurnStarted),
//! [`ReloadResult`](gdtf_battle_sim::ReloadResult)) PLUS — for the shot OUTCOME lines — the
//! presenter's per-shot [`ShotImpactResolved`](gdtf_battle_presenter::ShotImpactResolved) signal
//! (GTW-328): that signal is emitted at each shot's OWN (staggered) impact rather than on the
//! fire-frame `ShotFired` drain, so a burst's outcome lines appear ONE PER IMPACT, in cadence,
//! not all at once. It resolves each [`Entity`] to a
//! [`LogName`](gdtf_battle_presenter::LogName) via
//! `Query<&`[`GangerName`](gdtf_battle_sim::GangerName)`>`, builds the matching
//! [`CombatLogEvent`](gdtf_battle_presenter::CombatLogEvent), classifies it with the shared
//! slice-2 [`classify_log_event`](gdtf_battle_presenter::classify_log_event), and APPENDS each
//! resulting [`LogLine`](gdtf_battle_presenter::LogLine) as a UI text node under
//! [`CombatLogRoot`]. After appending, it FIFO-despawns the OLDEST lines so the visible count
//! never exceeds the tuned [`max_visible_lines`](super::super::tuning::CombatLogTuning).
//!
//! [`fade_combat_log_lines`] runs in `Update` (unguarded — it self-gates on the lines existing):
//! it ticks each line's [`LogLineFade`] clock, fades its [`UiTextColor`] alpha across the tail
//! of its life, and despawns the line the frame its clock finishes (the UI-space analogue of the
//! presenter's world-space `animate_floating_text`).

use bevy::{
    color::Alpha,
    ecs::system::SystemParam,
    prelude::*,
    scene::{CommandsSceneExt, bsn},
    text::{FontSize, FontWeight, LineHeight, TextColor as UiTextColor, TextFont},
    ui::{ComputedNode, Node, PositionType, Val},
};
use gdtf_battle_presenter::{
    CombatLogEvent, FctEmphasis, LogLine, LogName, ShotImpactResolved, classify_log_event,
};
use gdtf_battle_sim::{
    FireDeclaration, GangerName, MovementOccurred, PlayerFaction, ReloadResult, TurnStarted,
};
use gdtf_ui::theme::GdtfTheme;

use crate::states::running::game::battlescape::combat_log::{
    components::{CombatLogLine, CombatLogRoot, LineSlide, LogLineFade, PanelHeightAnim},
    tuning::CombatLogTuning,
};

/// The five combat-event [`MessageReader`]s [`update_combat_log`] drains, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate (the
/// sim's `FireSignals` / `BattleGridsParam` grouping precedent).
///
/// Four are the event-driven SIM signals (fire declaration / movement / turn / reload); the fifth
/// is the presenter's per-shot [`ShotImpactResolved`](gdtf_battle_presenter::ShotImpactResolved)
/// signal, drained for the shot OUTCOME lines so they appear at each shot's staggered impact
/// (GTW-328), not on the fire frame. A transparent system-param bundle of the five named message
/// buffers — not itself a wrapped domain value.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct CombatLogReaders<'w, 's> {
    /// The per-REQUEST shot declaration ("<name> fired <mode> at <target>").
    fire:     MessageReader<'w, 's, FireDeclaration>,
    /// The per-step movement ("<name> moved <from> -> <to>").
    movement: MessageReader<'w, 's, MovementOccurred>,
    /// The turn boundary ("— Player/Enemy turn —").
    turn:     MessageReader<'w, 's, TurnStarted>,
    /// The reload resolution ("<name> reloaded" / "<name>: no TU").
    reload:   MessageReader<'w, 's, ReloadResult>,
    /// The per-shot OUTCOME — the presenter's `ShotImpactResolved` signal, emitted at each shot's
    /// own (staggered) impact (GTW-328), NOT the fire-frame `ShotFired` drain. So a burst yields
    /// its damage/miss/wound/down lines ONE PER IMPACT, in cadence, instead of all at once.
    impact:   MessageReader<'w, 's, ShotImpactResolved>,
}

/// Resolve an [`Entity`] to a [`LogName`] via the ganger-name query, falling back to a generic
/// label for an unnamed / unresolvable actor (so a name-less ganger still logs a readable line
/// rather than nothing).
fn name_of(entity: Entity, names: &Query<&GangerName>) -> LogName {
    names
        .get(entity)
        .map_or_else(|_| LogName::new("Someone"), LogName::from_ganger)
}

/// Drain the five combat-event messages, classify each into log lines, and append them under
/// the combat-log root, FIFO-trimming to the tuned visible cap.
///
/// Runs in `Update` gated on the live-battle witness (the caller's run-condition). For each
/// drained message it resolves the actor / target [`Entity`] to a [`LogName`] (a shot outcome's
/// [`ShotImpactResolved`](gdtf_battle_presenter::ShotImpactResolved) carries the shooter name only
/// for the explicit miss line), builds the resolved [`CombatLogEvent`], classifies it with the
/// shared [`classify_log_event`], and spawns one UI text line per [`LogLine`] under
/// [`CombatLogRoot`]. It then despawns the OLDEST lines (by child order) until the visible count
/// is within [`max_visible_lines`](CombatLogTuning::max_visible_lines).
///
/// No-ops cleanly if the log root / theme are absent (`bevy-traps.md` #1 — present in a live
/// battle); the tuning is DEFAULTED if its async RON has not resolved yet, so a combat event
/// still logs at the sane defaults. Param-only (`bevy-traps.md` #7): [`Commands`], the message
/// readers, the name + root + line queries, the theme + tuning + player-faction reads.
#[expect(
    clippy::too_many_arguments,
    reason = "the combat log composes Commands, grouped readers, three queries, and three \
              resource reads to resolve + classify + append + trim in one pass; the readers \
              are already grouped into one SystemParam (the sim FireSignals precedent)"
)]
pub(in crate::states::running::game::battlescape) fn update_combat_log(
    mut commands: Commands,
    mut readers: CombatLogReaders,
    names: Query<&GangerName>,
    root: Query<Entity, With<CombatLogRoot>>,
    mut lines: Query<(Entity, &ComputedNode, &mut LineSlide), With<CombatLogLine>>,
    theme: Option<Res<GdtfTheme>>,
    tuning: Option<Res<CombatLogTuning>>,
    player: Option<Res<PlayerFaction>>,
) {
    let (Ok(root), Some(theme)) = (root.single(), theme) else {
        // No log container / theme yet — drain the readers so a pre-spawn event is not replayed
        // once the log exists, then bail (the lines would have nowhere to go).
        readers.fire.clear();
        readers.movement.clear();
        readers.turn.clear();
        readers.reload.clear();
        readers.impact.clear();
        return;
    };
    // The tuning's RON resolve is async; default it (TTL / fade / cap) if it has not resolved yet
    // so a combat event still logs at the sane defaults (it re-tunes live on resolve / hot-reload).
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);

    // Build the resolved combat-log events in arrival order across the five buffers. Each
    // resolves its Entity handle(s) to a LogName before leaving this World-touching layer, so
    // the shared classifier stays World-free (slice 2).
    let mut events: Vec<CombatLogEvent> = Vec::new();
    for declaration in readers.fire.read() {
        events.push(CombatLogEvent::FireDeclaration {
            actor:  name_of(declaration.shooter, &names),
            target: declaration.target.map(|t| name_of(t, &names)),
            mode:   declaration.mode,
        });
    }
    for movement in readers.movement.read() {
        events.push(CombatLogEvent::MovementOccurred {
            actor: name_of(movement.actor, &names),
            from:  movement.from,
            to:    movement.to,
        });
    }
    for impact in readers.impact.read() {
        // GTW-328: each shot's outcome line is built at its OWN (staggered) impact — when the
        // presenter emits `ShotImpactResolved` as the bolt lands — not on the fire-frame
        // `ShotFired` drain, so a burst's lines appear one-per-impact in cadence.
        events.push(CombatLogEvent::ShotOutcome {
            actor:  name_of(impact.shooter, &names),
            report: impact.report,
        });
    }
    for reload in readers.reload.read() {
        events.push(CombatLogEvent::ReloadResult {
            actor:   name_of(reload.actor, &names),
            outcome: reload.outcome,
        });
    }
    if let Some(player) = player {
        for turn in readers.turn.read() {
            events.push(CombatLogEvent::TurnStarted {
                now_active: turn.now_active,
                player:     *player,
            });
        }
    } else {
        // No player faction resolved — a turn boundary cannot be labelled Player/Enemy, so drop
        // it (drain so it is not replayed). The other events still log.
        readers.turn.clear();
    }

    // Classify each event into its lines and append a UI text node per line under the root,
    // collecting the freshly-spawned line ids (in append / chronological order) so the trim can
    // include them — the deferred new lines are NOT yet in the `lines` query this frame.
    let mut appended: Vec<Entity> = Vec::new();
    for event in &events {
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
/// `existing` query, so they are threaded in explicitly). If that total exceeds `cap`, the first
/// `overflow` of that combined order — the oldest — are despawned (FIFO). A `cap` of `0` is
/// treated as unbounded (despawn nothing) so a malformed tuning value cannot blank the log.
///
/// On a removal, each removed line's logical height (its [`ComputedNode`] size scaled by the
/// inverse scale factor) is summed and that freed height is ADDED to every SURVIVING prior
/// line's [`LineSlide`] offset, so the survivors hold their old screen position this frame and
/// then ease UP into the gap (`slide_combat_log_lines`) instead of snapping. (The just-appended
/// lines are not in this query yet — their slide is seeded at spawn.)
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

    // Bump every surviving line's slide by the freed height (it holds its old screen position
    // then eases up); skip the lines being removed.
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

/// The line-height multiple of a line's font size used to SEED its [`LineSlide`] appear-offset —
/// a fresh line is displaced ~one line-height below its slot, then slides UP into place (GTW-328
/// slice B). A framework-plumbing layout `const` (the carve-out): an estimate of the rendered
/// line height before the layout engine computes the real [`ComputedNode`], so the exact value is
/// cosmetic (the slide eases to `0` regardless).
const APPEAR_OFFSET_LINE_HEIGHTS: f32 = 1.3;

/// The line-height multiple of a combat-log line's font size — the full glyph box reserved per
/// line (ascenders + descenders + leading), set EXPLICITLY as [`LineHeight::Px`] and matched by the
/// line node's `min_height` so the clip never shaves a settled line (the cut-off fix).
///
/// A framework-plumbing layout `const` (the `ui-responsive-not-px` carve-out — a glyph-relative
/// multiple, not a fixed px). Bevy's [`LineHeight`] default is `RelativeToFont(1.2)`; this is a
/// touch more generous (`1.4`) so descenders + leading have headroom and a resting line is fully
/// legible rather than clipped flush at the panel's bottom edge.
const LINE_HEIGHT_SCALE: f32 = 1.4;

/// Spawn ONE combat-log line as a UI text node carrying the classified [`LogLine`]'s text +
/// color + emphasis-weight, plus the [`CombatLogLine`] marker, its [`LogLineFade`] clock, and its
/// [`LineSlide`] appear-offset (GTW-328 slice B).
///
/// Post-BSN spawn idiom (`bsn!` / `spawn_scene`): `Text` + `UiTextColor` ride the `bsn!` macro
/// inline; the runtime-valued `TextFont` (not `Unpin`) rides the `template(move |_| ..)`
/// closure escape hatch (the weapon-panel `spawn_text` precedent). The line draws at the tuned
/// [`line_font_pt`](CombatLogTuning::line_font_pt) — the log's OWN readable body size, kept larger
/// than the theme's body text (the readability fix) — and a lethal DOWN/DEAD line maps to a
/// [`FontWeight::BOLD`] + a size bump on top of that ([`emphasis_style`]). The [`CombatLogLine`]
/// marker, the three-phase [`LogLineFade`] clock (the tuned TTL + fade-in / fade-out seconds,
/// scaling from the line's own alpha), the [`LineSlide`] (seeded ~one line-height below its slot so
/// it slides UP into place), an explicit [`LineHeight::Px`] + matching `min_height` (so the full
/// glyph box is reserved and the clip never shaves a settled line — the cut-off fix), and a `Node`
/// with a `top` the slide system animates are `.insert`ed after the scene reserves the entity id.
/// The line spawns at `alpha 0` (its fade-in ramps it on) so it does not flash at full opacity for
/// a frame before the fade system runs. Returns the line entity.
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
    // Reserve the FULL glyph box (ascenders + descenders + leading) so the clip never shaves a
    // settled line — set explicitly rather than relying on bevy's RelativeToFont(1.2) default.
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
            UiTextColor(color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert((
            CombatLogLine,
            fade,
            slide,
            // The full glyph box per line (matches `min_height` below) so a settled line is never
            // shaved by the panel clip — descenders + leading have headroom (the cut-off fix).
            LineHeight::Px(line_box_px),
            // A RELATIVE node whose `top` carries the slide offset (the line keeps its flex-column
            // slot; `top` displaces it from that slot, eased to 0 by `slide_combat_log_lines`). Its
            // `min_height` reserves the full line box so the laid-out line never collapses shorter
            // than its glyphs (and the height target / clip account for the real line height).
            Node {
                position_type: PositionType::Relative,
                min_height: Val::Px(line_box_px),
                top: Val::Px(font_size * APPEAR_OFFSET_LINE_HEIGHTS),
                ..default()
            },
        ))
        .id()
}

/// The (weight, size) a classified line's [`FctEmphasis`] draws at — BOLD at a size bump for a
/// lethal DOWN/DEAD line, the base weight/size otherwise.
///
/// Mirrors the presenter's private `FctEmphasis::weight` / `font_size` mapping (which is not
/// `pub`), so the log line reads heavier for the lethal line on the bundled non-variable font.
fn emphasis_style(emphasis: FctEmphasis, base_pt: f32) -> (FontWeight, f32) {
    match emphasis {
        FctEmphasis::Bold => (FontWeight::BOLD, base_pt * BOLD_FONT_SCALE),
        FctEmphasis::Normal => (FontWeight::NORMAL, base_pt),
    }
}

/// The size multiplier a BOLD (lethal) combat-log line is drawn at, on top of the theme's body
/// size — the visible heavier-weight lever on the bundled non-variable font (the FCT precedent).
const BOLD_FONT_SCALE: f32 = 1.25;

/// Tick each combat-log line's three-phase fade clock; set its alpha along the fade-in / hold /
/// fade-out curve and despawn it the frame the clock finishes (GTW-328 slice B).
///
/// Runs in `Update`, unguarded — it self-gates on the [`LogLineFade`] lines existing (an empty
/// query is a no-op). Mirrors the presenter's world-space `animate_floating_text` but for UI
/// nodes (no rise — the lines slide via [`slide_combat_log_lines`]). Param-only
/// (`bevy-traps.md` #7): [`Commands`], the [`Time`] read, and the line query.
pub(in crate::states::running::game::battlescape) fn fade_combat_log_lines(
    mut commands: Commands,
    time: Res<Time>,
    mut lines: Query<(Entity, &mut UiTextColor, &mut LogLineFade)>,
) {
    let delta = time.delta();
    for (entity, mut color, mut fade) in &mut lines {
        if fade.advance(delta) {
            commands.entity(entity).despawn();
            continue;
        }
        // Set the alpha along the three-phase curve (fade-in / hold / fade-out), in place.
        color.0.set_alpha(fade.alpha());
    }
}

/// The per-second LERP rate above which the frame smoothing weight is treated as instant — guards
/// the `1 - exp(-rate * dt)` smoothing from a pathological tuning value. A framework-plumbing
/// `const` (the carve-out), not a domain value.
const MAX_LERP_RATE: f32 = 1_000.0;

/// The frame smoothing weight for an exponential ease at `rate` per second over `delta` seconds:
/// `1 - exp(-rate * dt)`, clamped to `0.0..=1.0`. Frame-rate independent — the same `rate`
/// settles in the same wall-clock time at any frame rate. A `rate` of `0` (or negative) yields
/// `0` (no movement); a very large `rate` saturates toward `1` (effectively instant).
fn lerp_factor(rate: f32, delta: f32) -> f32 {
    let rate = rate.clamp(0.0, MAX_LERP_RATE);
    (-rate * delta.max(0.0))
        .exp()
        .mul_add(-1.0, 1.0)
        .clamp(0.0, 1.0)
}

/// Ease each combat-log line's [`LineSlide`] offset toward its target slot (`0`) and write the
/// current offset into the line's [`Node::top`](bevy::ui::Node), so a line slides into place as
/// the stack reflows instead of SNAPPING (GTW-328 slice B).
///
/// Runs in `Update`, unguarded — it self-gates on the [`LineSlide`] lines existing (an empty
/// query is a no-op). The offset eases by [`lerp_factor`] at the tuned
/// [`line_lerp_rate`](CombatLogTuning::line_lerp_rate) per frame (frame-rate independent); the
/// tuning is DEFAULTED if its async RON has not resolved yet. Param-only (`bevy-traps.md` #7):
/// the [`Time`] read, the optional tuning read, and the line query.
pub(in crate::states::running::game::battlescape) fn slide_combat_log_lines(
    time: Res<Time>,
    tuning: Option<Res<CombatLogTuning>>,
    mut lines: Query<(&mut Node, &mut LineSlide), With<CombatLogLine>>,
) {
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);
    let factor = lerp_factor(*tuning.line_lerp_rate, time.delta_secs());
    for (mut node, mut slide) in &mut lines {
        slide.ease_toward_target(factor);
        // Write the eased offset into the line's relative `top` (its displacement from its slot).
        node.top = Val::Px(slide.current());
    }
}

/// Ease the combat-log panel's [`PanelHeightAnim`] toward its natural content height (the summed
/// logical line heights plus any active slide displacement plus the bottom clearance) and write it
/// into the root's [`Node::height`](bevy::ui::Node), so the panel grows / shrinks SMOOTHLY as lines
/// are added / removed instead of snapping (GTW-328 slice B).
///
/// Runs in `Update`, unguarded — it self-gates on the [`PanelHeightAnim`] root existing. It sums
/// the logical heights of the current [`CombatLogLine`] [`ComputedNode`]s (physical px scaled by
/// the inverse scale factor) and ADDS the largest active downward [`LineSlide`] offset — so while a
/// fresh line is still sliding UP into place (a positive `top`), the clipped panel is tall enough
/// to show it fully rather than shaving its bottom; as the slide settles to `0` that addend
/// vanishes and the panel eases back to the content height (GTW-328 slice B). It THEN adds a fixed
/// BOTTOM CLEARANCE (the tuned [`bottom_clearance_lines`](CombatLogTuning::bottom_clearance_lines)
/// times the line height) so the panel is always a touch taller than its content: the flex column is
/// top-aligned, so that surplus lands at the BOTTOM and lifts the NEWEST (bottommost) line's full
/// glyph box ABOVE the clip / bottom-bar edge — without it the bottommost line rests flush at the
/// clip and its descenders are shaved (the cut-off fix). The clearance is in the height target
/// rather than `Node::padding` because the themed panel's `padding` is re-derived from the theme by
/// `apply_theme` every repaint (it would clobber a spawn-time padding); the height is the panel's own
/// animated lever. The target is eased by [`lerp_factor`] at the tuned
/// [`height_lerp_rate`](CombatLogTuning::height_lerp_rate) per frame (frame-rate independent; tuning
/// DEFAULTED until its RON resolves), and written into the root [`Node`]. Param-only
/// (`bevy-traps.md` #7): the [`Time`] read, the optional tuning read, the line query (`ComputedNode`
/// + `LineSlide`), and the root query.
pub(in crate::states::running::game::battlescape) fn animate_combat_log_height(
    time: Res<Time>,
    tuning: Option<Res<CombatLogTuning>>,
    lines: Query<(&ComputedNode, &LineSlide), With<CombatLogLine>>,
    mut roots: Query<(&mut Node, &mut PanelHeightAnim), With<CombatLogRoot>>,
) {
    let Ok((mut node, mut anim)) = roots.single_mut() else {
        // No log root (not in a battle / not spawned yet) — nothing to animate.
        return;
    };
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);
    // The natural content height: the sum of the visible lines' LOGICAL heights.
    let content: f32 = lines
        .iter()
        .map(|(computed, _)| computed.size().y * computed.inverse_scale_factor())
        .sum();
    // The largest active downward slide offset — a line still easing UP into place is displaced
    // below its slot by this much, so the clipped panel must be this much taller to show it fully.
    // Settles to 0 as the slides finish, so the panel breathes back to the pure content height.
    let max_slide: f32 = lines
        .iter()
        .map(|(_, slide)| slide.current().max(0.0))
        .fold(0.0_f32, f32::max);
    // The fixed bottom clearance below the NEWEST line (a line-height multiple of the responsive
    // line font size) — so the top-aligned column leaves room at the bottom and the bottommost
    // line's full glyph box clears the clip / bottom-bar edge (the cut-off fix).
    let clearance = *tuning.line_font_pt * *tuning.bottom_clearance_lines;
    let target = height_target(content, max_slide, clearance);

    let factor = lerp_factor(*tuning.height_lerp_rate, time.delta_secs());
    anim.ease_toward(target, factor);
    node.height = Val::Px(anim.current());
}

/// The combat-log panel's animated-height TARGET (logical px): the summed line `content` height,
/// plus the largest active slide displacement `max_slide` (so a still-sliding line is shown fully),
/// plus a bottom `clearance` so the bottommost line clears the panel clip / bottom-bar edge (the
/// cut-off fix).
///
/// The clearance is applied ONLY when there is content (`content > 0`) so an EMPTY log collapses to
/// `0` rather than holding open a `clearance`-tall sliver of empty panel. With content present the
/// target STRICTLY EXCEEDS the content sum by the clearance: the flex column is top-aligned, so that
/// surplus lands at the bottom and lifts the newest line's full glyph box above the clip edge.
fn height_target(content: f32, max_slide: f32, clearance: f32) -> f32 {
    if content <= f32::EPSILON {
        // An empty log animates back to a collapsed 0 height — no clearance sliver held open.
        return 0.0;
    }
    content + max_slide + clearance
}

#[cfg(test)]
mod test {
    use super::height_target;

    /// A line that is close to `expected` (the float-cmp idiom; clippy `float_cmp` denies `==` on
    /// `f32`).
    fn approx(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() < 1e-4
    }

    /// With content present, the height target STRICTLY EXCEEDS the content sum by the bottom
    /// clearance (plus any slide) — the cut-off fix: the panel is taller than its lines so the
    /// top-aligned column leaves room below the newest line for its descenders to clear the clip.
    /// (Before the fix the target was exactly `content + max_slide`, leaving the bottommost line
    /// flush at the clip edge.)
    #[test]
    fn the_target_reserves_bottom_clearance_above_the_content_sum() {
        let content = 80.0;
        let clearance = 10.0;
        // Settled (no active slide): the target is the content PLUS the clearance — strictly taller.
        let target = height_target(content, 0.0, clearance);
        assert!(
            target > content,
            "with a positive clearance the panel must be taller than its content ({target} must \
             exceed {content}) so the bottommost line is not clipped",
        );
        assert!(
            approx(target, content + clearance),
            "the settled target is exactly content + clearance, got {target}",
        );
    }

    /// The clearance is ADDED ON TOP of an active slide displacement (they do not replace each
    /// other): while a fresh line still slides up, the panel reserves BOTH the slide room and the
    /// bottom clearance.
    #[test]
    fn the_clearance_stacks_with_an_active_slide() {
        let target = height_target(80.0, 6.0, 10.0);
        assert!(
            approx(target, 96.0),
            "content + slide + clearance are all reserved (80 + 6 + 10), got {target}",
        );
    }

    /// An EMPTY log (no content) collapses to `0` — the clearance is NOT held open as a sliver of
    /// empty panel, so an empty log animates fully closed.
    #[test]
    fn an_empty_log_collapses_to_zero_with_no_clearance_sliver() {
        let target = height_target(0.0, 0.0, 10.0);
        assert!(
            approx(target, 0.0),
            "an empty log targets 0 height (no clearance sliver), got {target}",
        );
    }
}
