//! The GTW-524 fall-impact FX — a one-frame impact flash at the landing cell and a
//! floating-combat-text `"Fell"` pop when a ganger drops a storey (or more).
//!
//! [`read_fall_occurred`] drains the sim's [`FallOccurred`](gdtf_battle_sim::falls::FallOccurred)
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
//!    the same one-shot pop the consequence reader uses. NO new FCT machinery. Its stacking
//!    slot is claimed from the shared lifetime-aware
//!    [`FctSlotAllocator`](super::fct::FctSlotAllocator) (GTW-794), so two falls landing on one
//!    cell across frames — or a fall over a live consequence / shot pop — stack instead of both
//!    reclaiming slot `0`.
//!
//! **C2 (tween)**: the faller's [`Changed<Position>`] (the sim's one-shot Position overwrite)
//! is consumed by the EXISTING [`advance_sprite_tweens`](super::super::ganger::tween) +
//! [`move_ganger_sprites`](crate::move_ganger_sprites) path verbatim —
//! no extension needed. The ganger's presenter sprite glides from the old storey's world
//! translation to the landing storey's world translation, reading as a downward drop (the
//! tween re-targets on every [`Changed<Position>`] regardless of whether the change is a walk
//! or an involuntary fall, so the drop is already covered).
//!
//! **C4 (gate)**: the flash spawn is gated
//! `run_if(resource_exists::<BattleInProgress>)` + the `Messages<FallOccurred>` buffer (the
//! sim's `FallsPlugin` registers it; the presenter registers it idempotently in
//! [`register_fx_flash_systems`](crate::TopDownRendererPlugin)). The FCT pop
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
use gdtf_battle_sim::{falls::FallOccurred, prelude::Position};

use super::{
    fct::{
        CombatText, FctEmphasis, FctSlotAllocator, FctValence, spawn_floating_text, valence_color,
    },
    readers::spawn_flash,
    roles::EffectRoles,
};
use crate::{FxTuning, TopDownAtlases, cell_to_world, fx::readers::fx_sprite, playback::Played};

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
/// `storeys: u8` is the INNER of [`StoreysFallen`](gdtf_battle_sim::falls::StoreysFallen) (via
/// `Deref`), carried here as a framework-plumbing scalar — the same raw storey count the
/// canonical [`CellLevel::level`](gdtf_battle_sim::metric::CellLevel::level) accessor recovers
/// from a key's `z` (GTW-565).
#[must_use]
fn fall_tint(storeys: u8) -> Color {
    // Alpha is a strictly-increasing relation to storeys: more storeys => a harder, more
    // opaque impact flash. Clamp into a visible floor (≥ 0.35) and a ceiling (1.0) so
    // the flash always reads.
    let alpha = (f32::from(storeys) / 3.0).clamp(0.35, 1.0);
    Color::srgba(0.95, 0.55, 0.10, alpha)
}

/// `Update` (`PresenterSystems::Overlay`): spawn a fall-impact flash + FCT pop per
/// [`FallOccurred`](gdtf_battle_sim::falls::FallOccurred) (GTW-524).
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
///    [`FctEmphasis::Normal`] and a stacking slot claimed from the shared lifetime-aware
///    [`FctSlotAllocator`](super::fct::FctSlotAllocator) (GTW-794) — the slot ABOVE every pop
///    still ALIVE on the landing cell. So two falls landing on one cell across frames (or a
///    fall over a live consequence / shot pop) stack instead of both reclaiming slot `0`,
///    replacing the former hardcoded slot `0`. A missing [`FxTuning`] leaves the FCT pop
///    unsprouted (the gate already requires its presence).
///
/// The faller's downward Position change (C2) is animated by the EXISTING
/// `advance_sprite_tweens` re-target path verbatim — no extension needed here.
///
/// ORDERING (`bevy-traps.md` #3): because it consumes the [`FctSlotAllocator`] it is registered
/// `.after(animate_floating_text)` — the allocator must count pops AFTER this frame's despawns
/// have flushed (GTW-794 / [`FctSlotAllocator`]).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`],
/// [`Res<EffectRoles>`], [`Res<FxTuning>`], [`MessageReader<FallOccurred>`], the
/// `Position` lookup, and the [`FctSlotAllocator`].
pub fn read_fall_occurred(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    tuning: Res<FxTuning>,
    mut falls: MessageReader<Played<FallOccurred>>,
    positions: Query<&Position>,
    allocator: FctSlotAllocator,
) {
    for msg in falls.read() {
        // Resolve the faller's current (landing) cell from its Position — the sim has
        // already written the landing storey before this reader sees the message, so the
        // Position is the ground truth for the flash anchor. Fail-closed: no Position →
        // no flash, no FCT pop, no panic.
        let Ok(pos) = positions.get(msg.ganger) else {
            continue;
        };
        // Position derefs to its CellLevel key (GTW-565) — the allocator's group key and, via
        // split, the flash/pop world anchor.
        let at = **pos;
        let (cell, level) = at.split();
        let world = cell_to_world(cell, level);

        // C1 — the impact flash. Data-driven `fall_impact` tile, no literal index.
        let storeys_raw = *msg.storeys; // StoreysFallen derefs to u8
        let tint = fall_tint(storeys_raw);
        if let Some(sprite) = fx_sprite(roles.fall_impact, tint, &atlases) {
            spawn_flash(&mut commands, sprite, world);
        }
        // (If the effects sheet is absent the flash is skipped FAIL-CLOSED.)

        // C3 — the FCT "Fell" pop, via the EXISTING spawn_floating_text helper. GTW-794: its
        // stacking slot is the one ABOVE every pop still alive on the landing cell, so
        // co-occurring falls (or a fall over a live consequence / shot pop) stack rather than
        // both reclaiming slot 0.
        let slot = allocator.next_slot(at);
        spawn_floating_text(
            &mut commands,
            CombatText::new("Fell"),
            valence_color(FctValence::Neutral),
            FctEmphasis::Normal,
            cell,
            level,
            slot,
            tuning.fct_ttl_seconds,
            tuning.fct_rise_rate,
        );
    }
}
