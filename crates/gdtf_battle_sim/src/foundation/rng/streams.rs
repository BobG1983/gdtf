//! Labeled `ChaCha12` streams derived from [`BattleSeed`](super::BattleSeed).

use rand::{SeedableRng, distr::uniform::SampleRange};
use rand_chacha::ChaCha12Rng;

use super::{
    derivation::{StreamLabel, fnv1a64},
    seeded::BattleSeed,
};

/// One resource newtype per stream, each with a fixed label.
macro_rules! impl_sim_stream {
    ($(#[$attr:meta])* $name:ident, $label:expr) => {
        $(#[$attr])*
        /// Deterministic RNG stream for this concern.
        #[derive(bevy::prelude::Resource)]
        pub struct $name(ChaCha12Rng);

        impl $name {
            /// Stream label bytes.
            pub const LABEL: StreamLabel = StreamLabel::new($label);

            /// Derive this stream from the battle root seed.
            #[must_use]
            pub fn from_root(root: BattleSeed) -> Self {
                let seed = fnv1a64(root, Self::LABEL.as_bytes());
                Self(ChaCha12Rng::seed_from_u64(seed.get()))
            }

            /// Mutable access to the underlying RNG.
            pub fn rng(&mut self) -> &mut impl rand::Rng {
                &mut self.0
            }

            /// Draw the next `u64`.
            pub fn next_u64(&mut self) -> u64 {
                use rand::RngExt as _;
                self.0.random()
            }

            /// Draw uniformly from `range`.
            pub fn random_range<T, R>(&mut self, range: R) -> T
            where
                T: rand::distr::uniform::SampleUniform,
                R: SampleRange<T>,
            {
                use rand::RngExt as _;
                self.0.random_range(range)
            }

            /// Draw from `range`, or its midpoint if the range is empty/inverted.
            pub fn random_range_or_midpoint(&mut self, range: core::ops::Range<f32>) -> f32 {
                *super::safe_draw::uniform_or_midpoint(&mut self.0, range)
            }
        }
    };
}

impl_sim_stream!(ShotRng, b"gdtf.rng.shot.v1");
impl_sim_stream!(SeverityRng, b"gdtf.rng.severity.v1");
impl_sim_stream!(LootRng, b"gdtf.rng.loot.v1");
impl_sim_stream!(InjuryRng, b"gdtf.rng.injury.v1");
impl_sim_stream!(ProcgenRng, b"gdtf.rng.procgen.v1");
impl_sim_stream!(DeploymentRng, b"gdtf.rng.deploy.v1");
impl_sim_stream!(ReactionRng, b"gdtf.rng.reaction.v1");
impl_sim_stream!(FightRng, b"gdtf.rng.fight.v1");
