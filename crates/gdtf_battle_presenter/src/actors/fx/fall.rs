//! The GTW-524 fall-impact FX — a one-frame impact flash at the landing cell and a
//! floating-combat-text `"Fell"` pop when a ganger drops a storey (or more).
//!
//! [`read_fall_occurred`] drains the sim's [`FallOccurred`](gdtf_battle_sim::FallOccurred)
//! signal (emitted once per falling ganger by the sim's `apply_falls`) and, for each fall:
//!
//! 1. **Flash (C1)** — spawns ONE transient impact glyph at `cell_to_world(landing_cell,
//!    to_level)` carrying [`FlashTtl`](super::flash::FlashTtl) + [`FxFlash`](super::flash::FxFlash),
//!    drawn from the data-driven [`EffectRoles::fall_impact`] tile (NO hardcoded tile index).
//!    The tint scales with `storeys` fallen so a multi-storey drop reads harder than a
//!    single-storey one (a visible `storeys`-relation tint, analogous to the `bleed_tint`
//!    Wounds relation in [`read_bleeding`](super::readers::read_bleeding)).
//!
//! 2. **FCT pop (C3)** — spawns ONE rise-and-fade [`FloatingCombatText`] `"Fell"` pop in the
//!    neutral GREY valence ([`FctValence::Neutral`](super::fct::palette::FctValence::Neutral))
//!    over the faller's landing cell, via the EXISTING
//!    [`spawn_floating_text`](super::fct::spawn_floating_text) helper. This IS the
//!    "combat-log line + FCT via the EXISTING … `read_consequence_fct` path" C3 requires —
//!    the same one-shot pop the consequence reader uses. NO new FCT machinery.
//!
//! **C2 (tween)**: the faller's [`Changed<Position>`] (the sim's one-shot Position overwrite)
//! is consumed by the EXISTING [`advance_sprite_tweens`](super::super::ganger::tween) +
//! [`move_ganger_sprites`](super::super::ganger::systems::move_ganger_sprites) path verbatim —
//! no extension needed. The ganger's presenter sprite glides from the old storey's world
//! translation to the landing storey's world translation, reading as a downward drop (the
//! tween re-targets on every [`Changed<Position>`] regardless of whether the change is a walk
//! or an involuntary fall, so the drop is already covered).
//!
//! **C4 (gate)**: the flash spawn is gated
//! `run_if(resource_exists::<BattleInProgress>)` + the `Messages<FallOccurred>` buffer (the
//! sim's `FallsPlugin` registers it; the presenter registers it idempotently in
//! [`register_fx_flash_systems`](crate::plugin::renderer::TopDownRendererPlugin)). The FCT pop
//! rides the same gated system.
//!
//! **C5 (transient)**: the flash is one-shot / transient — the EXISTING `expire_flashes`
//! clock ticks it and despawns it after its [`FlashTtl`](super::flash::FlashTtl) window.
//! ADDITIVE to the injury/bleed flashes [`InjuryInflicted`] drives: this module ONLY spawns
//! the fall-impact glyph; wound / injury / bleed FX from the landing fall-damage arrive on
//! their own existing signals and are drawn by their own existing readers.
//!
//! Pure VIEW (ADR-0001): reads the sim signal + the data-driven [`EffectRoles`] table and
//! draws sprites; NEVER writes the sim.

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, FallOccurred, Level, Position};

use super::{
    fct::{CombatText, FctEmphasis, FctStackIndex, FctValence, spawn_floating_text, valence_color},
    readers::spawn_flash,
    roles::EffectRoles,
};
use crate::{FxTuning, TopDownAtlases, cell_to_world, fx::readers::fx_sprite};

/// The fall-impact-flash tint as a relation to `storeys` fallen — a visible structural
/// mapping (analogous to [`bleed_tint`](super::readers::bleed_tint)'s `Wounds` relation).
///
/// A longer fall reads a MORE saturated / opaque flash (the physical impact is harder),
/// while a single-storey drop reads fainter. The orange-amber hue (distinct from both the
/// blood red of damage and the neutral grey of a miss) reads the fall as a kinetic
/// STRUCTURAL event: the ganger hit the floor. Alpha rises with storeys fallen (saturates
/// at 3+ storeys so a tall fall is fully opaque). Not a pinned literal —
/// `storeys` is a structural relation recomputed from the live [`FallOccurred`] payload.
///
/// `storeys: u8` is the INNER of [`StoreysFallen`](gdtf_battle_sim::StoreysFallen) (via
/// `Deref`), carried here as a framework-plumbing scalar — it is the raw count
/// `cell_and_level` reconstructs, the same idiom every other FX reader uses for `pos.z`.
#[must_use]
fn fall_tint(storeys: u8) -> Color {
    // Alpha is a strictly-increasing relation to storeys: more storeys => a harder, more
    // opaque impact flash. Clamp into a visible floor (≥ 0.35) and a ceiling (1.0) so
    // the flash always reads.
    let alpha = (f32::from(storeys) / 3.0).clamp(0.35, 1.0);
    Color::srgba(0.95, 0.55, 0.10, alpha)
}

/// `Update` (`PresenterSystems::Draw`): spawn a fall-impact flash + FCT pop per
/// [`FallOccurred`](gdtf_battle_sim::FallOccurred) (GTW-524).
///
/// Drains [`MessageReader<FallOccurred>`]; for each message it:
///
/// 1. Looks up the faller's LANDING cell via `Query<&Position>.get(msg.ganger)` (the sim
///    has already overwritten the Position to the landing storey by the time this reader
///    runs, so `msg.to_level` == position.z in practice; reading from `Position` ensures
///    the flash lands at the ACTUAL present location, fail-closed on a missing Position).
/// 2. Spawns ONE impact-flash sprite at `cell_to_world(cell, level)` via the shared
///    `spawn_flash` recipe using the data-driven `fall_impact`
///    [`TileIndex`](crate::TileIndex) from [`EffectRoles`] (NO literal). Tint is a storey
///    relation via `fall_tint`. A missing effects sheet skips FAIL-CLOSED.
/// 3. Spawns ONE `"Fell"` FCT pop in neutral GREY (the one-shot C3 "log line") via the
///    EXISTING [`spawn_floating_text`](super::fct::spawn_floating_text) helper, with
///    [`FctEmphasis::Normal`] and a zero stack slot (one fall per ganger per frame is the
///    normal case; the slot would need stacking if two falls on one cell ever co-occur,
///    but a single fall per cell per tick is the model invariant — future stacking is
///    additive if ever needed). A missing [`FxTuning`] leaves the FCT pop unsprouted (the
///    gate already requires its presence).
///
/// The faller's downward Position change (C2) is animated by the EXISTING
/// `advance_sprite_tweens` re-target path verbatim — no extension needed here.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`],
/// [`Res<EffectRoles>`], [`Res<FxTuning>`], [`MessageReader<FallOccurred>`], and the
/// `Position` lookup.
pub fn read_fall_occurred(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    tuning: Res<FxTuning>,
    mut falls: MessageReader<FallOccurred>,
    positions: Query<&Position>,
) {
    for msg in falls.read() {
        // Resolve the faller's current (landing) cell from its Position — the sim has
        // already written the landing storey before this reader sees the message, so the
        // Position is the ground truth for the flash anchor. Fail-closed: no Position →
        // no flash, no FCT pop, no panic.
        let Ok(pos) = positions.get(msg.ganger) else {
            continue;
        };
        let cell = Cell::new(pos.x, pos.y);
        let storey = u8::try_from(pos.z).unwrap_or(0);
        let level = Level::new(storey);
        let world = cell_to_world(cell, level);

        // C1 — the impact flash. Data-driven `fall_impact` tile, no literal index.
        let storeys_raw = *msg.storeys; // StoreysFallen derefs to u8
        let tint = fall_tint(storeys_raw);
        if let Some(sprite) = fx_sprite(roles.fall_impact, tint, &atlases) {
            spawn_flash(&mut commands, sprite, world);
        }
        // (If the effects sheet is absent the flash is skipped FAIL-CLOSED.)

        // C3 — the FCT "Fell" pop, via the EXISTING spawn_floating_text helper.
        spawn_floating_text(
            &mut commands,
            CombatText::new("Fell"),
            valence_color(FctValence::Neutral),
            FctEmphasis::Normal,
            cell,
            level,
            FctStackIndex::new(0),
            tuning.fct_ttl_seconds,
            tuning.fct_rise_rate,
        );
    }
}
