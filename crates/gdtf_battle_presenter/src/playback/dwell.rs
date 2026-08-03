//! Every value is a named newtype with a `DEFAULT` and `#[serde(default)]`, loaded through
use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

macro_rules! dwell_seconds {
    ($(#[$meta:meta])* $name:ident, $default:expr, $doc:expr) => {
        #[doc = $doc]
                                /// [`Default`]. `#[serde(default)]` on the field means an absent `.ron` entry
                #[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
        #[serde(transparent)]
        pub struct $name(f32);

        impl $name {
                        pub const DEFAULT: f32 = $default;

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

#[derive(Resource, Debug, Clone, PartialEq, Default, Deserialize, TypePath)]
#[serde(default)]
pub struct PlaybackTuning {
        pub round_seconds:         RoundSeconds,
        pub reaction_beat_seconds: ReactionBeatSeconds,
        pub fire_beat_seconds:     FireBeatSeconds,
        pub step_seconds:          StepSeconds,
        pub posture_seconds:       PostureSeconds,
        pub reload_seconds:        ReloadSeconds,
        pub consequence_seconds:   ConsequenceSeconds,
        pub life_change_seconds:   LifeChangeSeconds,
        pub turn_beat_seconds:     TurnBeatSeconds,
        pub minor_seconds:         MinorSeconds,
        pub impact_cap_seconds:    ImpactCapSeconds,
}

const PLAYBACK_TUNING_RON_PATH: &str = "core_tuning/playback.tuning.ron";

pub(super) fn register_playback_tuning_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<PlaybackTuning>(PLAYBACK_TUNING_RON_PATH);
}
