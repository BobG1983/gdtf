//! The per-stream SEED DERIVATION — [`fnv1a64`] + [`StreamLabel`] (GTW-14, §A).
//!
//! Given a [`BattleSeed`](super::seeded::BattleSeed) `root`, each stream's seed is
//! `fnv1a64( root.to_le_bytes() ++ LABEL_bytes )` → `ChaCha12Rng::seed_from_u64`.
//! The derivation lives in its own leaf (split from `streams`, GTW-640/644 rider)
//! so the "how a seed is derived" concern changes independently of the "what draw
//! surface a stream exposes" concern. The stream types themselves (and the
//! `impl_sim_stream!` macro that stamps them) live in `streams`.

use super::seeded::BattleSeed;

// ── FNV-1a-64 derivation ────────────────────────────────────────────────────

/// The FNV-1a-64 offset basis (the FNV spec's fixed constant).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// The FNV-1a-64 prime (the FNV spec's fixed constant).
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a-64 of a fixed 8-byte root seed (little-endian) concatenated with a
/// label slice — the per-stream seed derivation (GTW-14, §A).
///
/// Hand-rolled as a `const fn` so the derivation is entirely specified here and
/// does NOT rely on [`std::hash::DefaultHasher`] (whose output is not
/// stability-guaranteed across Rust releases or targets). Only `wrapping_mul` /
/// `^` are needed; the arithmetic is identical on every platform.
///
/// The byte stream fed to FNV is:
/// `root.to_le_bytes()` (always exactly 8 bytes) `++` `label` (fixed ASCII)
///
/// — so the concatenation is unambiguous without a length prefix. The resulting
/// `u64` is fed to `SeedableRng::seed_from_u64` in each stream's constructor.
pub(super) const fn fnv1a64(root: BattleSeed, label: &[u8]) -> u64 {
    let root_bytes = root.get().to_le_bytes();
    // Feed the 8 root bytes first.
    let mut hash = FNV_OFFSET;
    let mut i = 0usize;
    while i < root_bytes.len() {
        hash = hash ^ (root_bytes[i] as u64);
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    // Then the label bytes.
    let mut j = 0usize;
    while j < label.len() {
        hash = hash ^ (label[j] as u64);
        hash = hash.wrapping_mul(FNV_PRIME);
        j += 1;
    }
    hash
}

// ── Stream-label newtype ─────────────────────────────────────────────────────

/// A stable, versioned byte-label identifying one RNG stream in the
/// [`fnv1a64`] root-seed derivation.
///
/// Distinct labels → independent per-stream seeds (GTW-14, §A). Each label is a
/// fixed versioned ASCII constant (`.v1` suffix) so a deliberate re-tune bumps
/// one label without disturbing others. The inner `&'static [u8]` is private;
/// the derivation reads it only through [`StreamLabel::as_bytes`].
pub struct StreamLabel(&'static [u8]);

impl StreamLabel {
    /// Construct a stream label from a fixed `'static` byte slice.
    ///
    /// The one constructor; the label is always a compile-time constant so no
    /// runtime allocation is needed and the `const fn` carries zero overhead.
    #[must_use]
    pub const fn new(b: &'static [u8]) -> Self {
        Self(b)
    }

    /// The raw bytes of this label — fed to [`fnv1a64`] as the second segment.
    ///
    /// Private to the `rng` module; callers use the stream's `from_root`
    /// constructor which combines label + root through `fnv1a64` internally.
    pub(super) const fn as_bytes(&self) -> &[u8] {
        self.0
    }
}
