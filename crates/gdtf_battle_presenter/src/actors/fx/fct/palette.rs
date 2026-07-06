//! The floating-combat-text VALENCE palette — the one place a pop's MEANING maps to a
//! color.
//!
//! A floating-combat-text pop signals one of a few combat *valences* (damage taken, a
//! wound / status change, a neutral miss, a lethal down / death). This module is the
//! single home for the valence → color mapping ([`FctValence`] / [`valence_color`]) plus
//! the [`Severity`](gdtf_battle_sim::severity::Severity)-tier → amber-family ramp
//! ([`severity_color`]) the reader slices (3-4) will feed their pops through. Keeping it
//! here — separate from the spawn/animate primitive in [`text`](super::text) — means a
//! later palette retune touches ONE file, and the primitive stays a pure
//! `(text, color, emphasis, cell, level, stack)` spawner.
//!
//! The palette is deliberately presenter-owned (it picks the literal sRGB swatches the FX
//! layer draws), NOT the `gdtf_ui` `GdtfTheme`: the theme's roles (panel / button / title /
//! text) are UI-chrome colors with no combat-valence meaning, and the presenter does not
//! depend on `gdtf_ui` at all. The FCT world layer wants saturated, legible-over-terrain
//! swatches rather than the muted chrome palette. The swatches below are the documented FCT
//! palette; a later ticket can migrate them to a hot-reloadable `.ron` the way the FX tuning
//! table did.

use bevy::prelude::*;
use gdtf_battle_sim::severity::Severity;

/// RED — damage taken. The dominant FCT valence: an HP-loss number a hit dealt.
///
/// A saturated red that reads over both light floor and dark cover terrain. The lethal
/// [`FctValence::Lethal`] pop reuses this hue (drawn bold + larger by the reader, via
/// `FctEmphasis::Bold`), so a down / death stays in the same "blood" family as ordinary
/// damage rather than introducing a new hue.
const DAMAGE_RED: Color = Color::srgb(0.90, 0.13, 0.10);

/// AMBER — a wound / status change (the [`Severity`] family). The mid-valence pop: a
/// limb wounded, a status applied — distinct from the raw HP-loss red.
///
/// The BASE of the [`severity_color`] amber ramp (the lightest, lowest-tier wound). The
/// ramp darkens toward orange-red as the severity tier climbs (see [`severity_color`]).
const WOUND_AMBER: Color = Color::srgb(0.95, 0.70, 0.15);

/// AMBER (deep) — the TOP of the [`severity_color`] ramp (a [`Severity::Critical`]
/// wound), an orange-red one tier shy of the lethal red.
///
/// [`severity_color`] interpolates from [`WOUND_AMBER`] toward this swatch across the
/// wounding tiers so a worse wound reads hotter; a [`Severity::Fatal`] tier instead jumps
/// to the lethal red (it is a death, not a wound).
const WOUND_AMBER_DEEP: Color = Color::srgb(0.96, 0.42, 0.08);

/// GREY — a neutral / miss pop. The quietest valence: a clean miss or a no-consequence
/// event that still wants a floating marker (e.g. a "miss" tag).
///
/// A desaturated light grey so a miss reads as background noise next to the saturated
/// damage / wound pops — present, but not shouting.
const NEUTRAL_GREY: Color = Color::srgb(0.72, 0.72, 0.74);

/// COWED BLUE-GREY — a suppression pop (GTW-526 C8). A morale / status valence for a ganger
/// pinned down by incoming fire, distinct from the damage RED, wound AMBER, and neutral GREY.
///
/// A muted, desaturated blue-grey that reads as "cowed / lost its nerve" — the same
/// colour-drained family the suppressed sprite tint uses, so the transient `"SUPPRESSED"` pop
/// and the persistent sprite desaturation read as one signal. Cool + dim (not a shouting
/// saturated hue) because suppression is a state the unit is UNDER, not a hit it took.
const SUPPRESSED_BLUE_GREY: Color = Color::srgb(0.45, 0.55, 0.72);

/// TOXIC GREEN — a damage-over-time tick pop (GTW-544). A per-turn attrition valence for the
/// flat HP a burning / caustic DOT drains each round, distinct from the raw-hit damage RED,
/// the wound AMBER, the neutral GREY, the lethal RED, and the cowed suppression blue-grey.
///
/// A saturated sickly green — the genre "poison / acid / plasma-burn" hue — so a DOT tick reads
/// as its own recurring attrition signal rather than being mistaken for a fresh weapon hit
/// (RED) or a bleed status tag (AMBER). It rides its own valence because a DOT tick is a
/// deterministic per-round drain, not a shot's impact number.
const DOT_TOXIC_GREEN: Color = Color::srgb(0.35, 0.82, 0.20);

/// HAZARD ORANGE — an area-damage-field tick pop (GTW-545). A per-turn attrition valence for the
/// flat HP a persistent damage ZONE (a toxic-waste pool, an electrified floor, a patch of burning
/// ground) drains off the ganger standing in it each round, distinct from the raw-hit damage RED,
/// the wound AMBER, the neutral GREY, the lethal RED, the cowed suppression blue-grey, and the
/// DOT toxic green.
///
/// A saturated hazard orange — the genre "industrial hazard / danger zone" hue — so a field tick
/// reads as its own recurring environmental-attrition signal, distinct from a fresh weapon hit
/// (RED) and from the DOT's toxic green (a field is a ZONE you stand in, not an affliction you
/// carry). It rides its own valence because a field tick is a deterministic per-round zone drain,
/// not a shot's impact number nor a carried affliction's tick.
const FIELD_HAZARD_ORANGE: Color = Color::srgb(0.95, 0.50, 0.10);

/// The combat VALENCE a floating-combat-text pop signals — the presenter's own neutral
/// category that decides the pop's color.
///
/// A named domain enum (no bare color / tag): the reader slices (3-4) classify each
/// [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired) consequence into one of these, and
/// [`valence_color`] turns it into the swatch the pop is drawn in. Exhaustive: exactly the
/// valences the FCT palette distinguishes. A [`Severity`]-tiered wound picks its exact
/// amber via [`severity_color`] rather than the flat [`Status`](FctValence::Status) swatch,
/// but the flat swatch remains the fallback for a status pop that carries no severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FctValence {
    /// Damage taken — an HP-loss number. Drawn [`DAMAGE_RED`].
    Damage,
    /// A status change / caution with no severity tier to scale by — never a
    /// severity-bearing wound (those pick their amber via [`severity_color`]). Drawn the
    /// flat [`WOUND_AMBER`] base.
    Status,
    /// A neutral / miss pop. Drawn [`NEUTRAL_GREY`].
    Neutral,
    /// A lethal outcome — a ganger went Downed or Dead. Drawn [`DAMAGE_RED`] (the reader draws
    /// it bold + larger via `FctEmphasis::Bold` so it reads as the heaviest pop in the blood
    /// family).
    Lethal,
    /// A suppression event — a ganger was pinned down by incoming fire (GTW-526 C8). Drawn the
    /// cowed [`SUPPRESSED_BLUE_GREY`], the colour-drained family the suppressed sprite tint
    /// shares, so the `"SUPPRESSED"` pop and the desaturated sprite read as one signal.
    Suppressed,
    /// A damage-over-time tick — a burning / caustic affliction drained flat HP this round
    /// (GTW-544). Drawn the toxic [`DOT_TOXIC_GREEN`] so the recurring attrition reads as its
    /// own signal, distinct from a fresh weapon hit (RED) or a bleed status tag (AMBER).
    Dot,
    /// An area-damage-field tick — a persistent damage ZONE (toxic pool / electrified floor /
    /// burning ground) drained flat HP off the ganger standing in it this round (GTW-545). Drawn
    /// the hazard [`FIELD_HAZARD_ORANGE`] so the recurring environmental attrition reads as its
    /// own signal, distinct from a fresh weapon hit (RED) and from the DOT toxic green.
    Field,
}

/// The FCT swatch for a combat [`FctValence`] — the one valence → color mapping.
///
/// [`Damage`](FctValence::Damage) and [`Lethal`](FctValence::Lethal) share the
/// [`DAMAGE_RED`] blood family (lethal is drawn heavier by the caller via `FctEmphasis::Bold`,
/// same hue);
/// [`Status`](FctValence::Status) is the flat [`WOUND_AMBER`] base; [`Neutral`](FctValence::Neutral)
/// is [`NEUTRAL_GREY`]; [`Suppressed`](FctValence::Suppressed) is the cowed
/// [`SUPPRESSED_BLUE_GREY`]; [`Dot`](FctValence::Dot) is the toxic [`DOT_TOXIC_GREEN`];
/// [`Field`](FctValence::Field) is the hazard [`FIELD_HAZARD_ORANGE`]. A severity-bearing wound
/// should instead call [`severity_color`] to scale within the amber family.
#[must_use]
pub const fn valence_color(valence: FctValence) -> Color {
    match valence {
        FctValence::Damage | FctValence::Lethal => DAMAGE_RED,
        FctValence::Status => WOUND_AMBER,
        FctValence::Neutral => NEUTRAL_GREY,
        FctValence::Suppressed => SUPPRESSED_BLUE_GREY,
        FctValence::Dot => DOT_TOXIC_GREEN,
        FctValence::Field => FIELD_HAZARD_ORANGE,
    }
}

/// The FCT amber swatch for a [`Severity`] tier — the wound-family ramp.
///
/// Maps the [`Severity`] ladder into the amber family: a [`None`](Severity::None) graze
/// (HP loss only, no wound spent) reads NEUTRAL (it is not a wound), the wounding tiers
/// ([`Minor`](Severity::Minor) → [`Critical`](Severity::Critical)) ramp from the light
/// [`WOUND_AMBER`] base toward the hot [`WOUND_AMBER_DEEP`] as the tier climbs (a worse
/// wound reads hotter), and a [`Fatal`](Severity::Fatal) hit jumps to the lethal
/// [`DAMAGE_RED`] (it is a death, not a wound). The ramp uses each tier's
/// [`Severity::rank`] so the mapping stays in lock-step with the sim's ladder order
/// without pinning any score magnitude.
#[must_use]
pub fn severity_color(severity: Severity) -> Color {
    match severity {
        // A graze costs no Wound (severity.rs §6) — it is HP loss, not a wound, so it
        // reads neutral rather than entering the amber wound family.
        Severity::None => NEUTRAL_GREY,
        // A death is the lethal blood-red, not a wound amber.
        Severity::Fatal => DAMAGE_RED,
        // The wounding tiers (Minor=1, Major=2, Critical=3) ramp light → hot amber. Map
        // the rank onto t ∈ [0, 1] across the three wounding tiers (Minor → 0, Critical
        // → 1) and lerp the base toward the deep swatch.
        wound => {
            /// The lowest wounding rank ([`Severity::Minor`]) — the ramp's `t = 0` anchor.
            const MIN_WOUND_RANK: f32 = 1.0;
            /// The span of wounding ranks (Minor=1 … Critical=3) the ramp interpolates
            /// across, so Critical lands at `t = 1`.
            const WOUND_RANK_SPAN: f32 = 2.0;
            let t = ((f32::from(wound.rank()) - MIN_WOUND_RANK) / WOUND_RANK_SPAN).clamp(0.0, 1.0);
            WOUND_AMBER.mix(&WOUND_AMBER_DEEP, t)
        }
    }
}
