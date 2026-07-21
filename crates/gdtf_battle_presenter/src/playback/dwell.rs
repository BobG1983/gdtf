//! [`PlaybackTuning`] — the hot-reloadable per-deed DWELLS the playback cursor holds each
//! act for (GTW-727 C20).
//!
//! Every value is a named newtype with a `DEFAULT` and `#[serde(default)]`, loaded through
//! the same generic hot-RON registration path `FxTuning` uses, so the whole feel of combat
//! playback is tunable from `assets/core_tuning/playback.tuning.ron` WITHOUT a rebuild —
//! which is the only practical way to find the right pacing, since "readable" is a
//! judgement made by watching, not by reasoning.
//!
//! **Dwells are read PER ACT, at hold time — never captured when an entry is enqueued.**
//! That is deliberate: a later skip / fast-forward affordance then becomes a rate
//! multiplier applied at the same read, rather than a restructure of how holds are built.

use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

/// Declares one dwell newtype: a serde-transparent `f32` seconds value with a private
/// inner, a `DEFAULT` constant, a `new` constructor and a `Default` impl.
///
/// Ten near-identical newtypes differing only in name, default and prose is exactly the
/// shape a declarative macro exists for; writing them out longhand would be 200 lines in
/// which a copy-paste slip (a default wired to the wrong constant) is invisible.
macro_rules! dwell_seconds {
    ($(#[$meta:meta])* $name:ident, $default:expr, $doc:expr) => {
        #[doc = $doc]
        ///
        /// Seconds. A named newtype (`no-bare-types.md`): the inner is PRIVATE, read
        /// through the derived [`Deref`] and built through [`new`](Self::new) /
        /// [`Default`]. `#[serde(default)]` on the field means an absent `.ron` entry
        /// degrades to the shipped default rather than failing the parse.
        #[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
        #[serde(transparent)]
        pub struct $name(f32);

        impl $name {
            /// The shipped default, in seconds.
            pub const DEFAULT: f32 = $default;

            /// Build this dwell from a duration in seconds.
            #[must_use]
            pub const fn new(seconds: f32) -> Self {
                Self(seconds)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self(Self::DEFAULT)
            }
        }
    };
}

dwell_seconds!(
    RoundSeconds,
    0.35,
    "How long the cursor beats after ONE round of a volley has landed, before playing the \
     next entry.\n\nThe default `0.35` is deliberately the value the retired \
     `InterShotSeconds` stagger used, so a burst reads at exactly the cadence it did \
     before — the mechanism moved from the projectile spawner to the cursor, the feel did \
     not."
);
dwell_seconds!(
    ReactionBeatSeconds,
    0.45,
    "How long the cursor beats on a REACTION-fire declaration — longer than an ordinary \
     one, because an out-of-turn interrupt is a surprise the player did not order and \
     needs a moment to be read as its own event rather than part of the act it \
     interrupted."
);
dwell_seconds!(
    FireBeatSeconds,
    0.15,
    "How long the cursor beats on an ordinary fire declaration before its first round \
     resolves — the pause between raising the weapon and the shot leaving it."
);
dwell_seconds!(
    StepSeconds,
    0.20,
    "How long the cursor beats on one walk step, so a multi-cell walk reads cell by cell."
);
dwell_seconds!(
    PostureSeconds,
    0.18,
    "How long the cursor beats on a posture change (turning to face, going prone, taking \
     aim, being suppressed)."
);
dwell_seconds!(
    ReloadSeconds,
    0.30,
    "How long the cursor beats on a resolved reload."
);
dwell_seconds!(
    ConsequenceSeconds,
    0.25,
    "How long the cursor beats on a consequence — the damage numbers, an injury, a fall, a \
     broken plate, a fresh affliction."
);
dwell_seconds!(
    LifeChangeSeconds,
    0.50,
    "How long the cursor beats on a life-state change (going down, dying, being revived) — \
     the longest ordinary dwell, because it is the most consequential thing that can \
     happen to a ganger."
);
dwell_seconds!(
    TurnBeatSeconds,
    0.60,
    "How long the cursor beats on a turn boundary, so the handover between sides reads as \
     a break rather than as more of the same."
);
dwell_seconds!(
    MinorSeconds,
    0.10,
    "How long the cursor beats on a minor fact that needs to be ordered but not dwelt on — \
     a refused move, an ammo readout change, a per-round bleed drain."
);
dwell_seconds!(
    ImpactCapSeconds,
    2.00,
    "The HARD upper bound on how long the cursor will wait for a round's projectile to fly \
     and land.\n\nA backstop, not a dwell: the projectile spawner legitimately spawns NO \
     bolt when its effects sheet is missing or too short, and a hot-reloadable projectile \
     velocity of zero would leave a bolt in flight forever. Without this cap either case \
     would hold the cursor — and therefore the input gate — indefinitely, with no in-battle \
     quit key to escape it. `2.0` comfortably exceeds a full-diagonal shot's flight time at \
     the shipped velocity, so it never truncates a real bolt."
);

/// The resolved playback-dwell table the cursor reads (GTW-727 C20).
///
/// Hot-reloadable through the generic hot-RON registration path
/// ([`register_playback_tuning_hot_ron`]): an edit
/// to the `.ron` overwrites this resource, so the very next hold uses the new value with no
/// rebuild. Also `init_resource`'d by the renderer plugin, so the cursor always has a table
/// even before (or without) the asset chain — a `MinimalPlugins` app with no `AssetServer`
/// skips the chain entirely and keeps the shipped defaults.
#[derive(Resource, Debug, Clone, PartialEq, Default, Deserialize, TypePath)]
#[serde(default)]
pub struct PlaybackTuning {
    /// The beat after one round of a volley lands.
    pub round_seconds:         RoundSeconds,
    /// The beat on a reaction-fire declaration.
    pub reaction_beat_seconds: ReactionBeatSeconds,
    /// The beat on an ordinary fire declaration.
    pub fire_beat_seconds:     FireBeatSeconds,
    /// The beat on one walk step.
    pub step_seconds:          StepSeconds,
    /// The beat on a posture change.
    pub posture_seconds:       PostureSeconds,
    /// The beat on a resolved reload.
    pub reload_seconds:        ReloadSeconds,
    /// The beat on a consequence.
    pub consequence_seconds:   ConsequenceSeconds,
    /// The beat on a life-state change.
    pub life_change_seconds:   LifeChangeSeconds,
    /// The beat on a turn boundary.
    pub turn_beat_seconds:     TurnBeatSeconds,
    /// The beat on a minor ordered fact.
    pub minor_seconds:         MinorSeconds,
    /// The hard cap on waiting for a projectile to land.
    pub impact_cap_seconds:    ImpactCapSeconds,
}

/// The path of the loose playback-tuning RON, relative to the asset source root.
const PLAYBACK_TUNING_RON_PATH: &str = "core_tuning/playback.tuning.ron";

/// Registers the [`PlaybackTuning`] hot-RON chain — ONE ext call onto the generic hot-RON
/// registration path, exactly as `register_fx_tuning_hot_ron` does for the FX table.
///
/// Self-gates on the [`AssetServer`](bevy::asset::AssetServer) (`bevy-traps.md` #1), so a
/// `MinimalPlugins` headless app is a no-op and keeps the `init_resource` defaults.
pub(super) fn register_playback_tuning_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<PlaybackTuning>(PLAYBACK_TUNING_RON_PATH);
}
