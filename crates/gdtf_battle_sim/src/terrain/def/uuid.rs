//! The **terrain-definition key** — [`TerrainUuid`], a stable UUID the unified
//! terrain model ([`TerrainDef`](super::TerrainDef)) is keyed by (GTW-484).

use bevy::{asset::uuid::Uuid, prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// A terrain definition's **stable key** — the UUID that identifies one
/// [`TerrainDef`](super::TerrainDef) across themes, prefabs, and the registry.
///
/// Per the GTW-476 redesign, terrain is referenced by a stable UUID (not by a
/// filename stem like the legacy [`TerrainName`](crate::terrain::piece::TerrainName))
/// so a definition can be renamed or moved without breaking the references that point
/// at it. The [`TerrainDefRegistry`](super::TerrainDefRegistry) keys definitions by
/// this value.
///
/// A UUID newtype (no-bare-types rule 1: a key is a domain value, not a bare `Uuid`).
/// The inner is the [`Uuid`] **re-exported by Bevy** at `bevy::asset::uuid` — gdtf
/// depends on the `uuid` crate ONLY through Bevy (no-bare-types: no direct dep on a
/// crate Bevy re-exports), the same path `gdtf_ui` already uses for weak asset
/// handles. Private inner + **derived** [`Deref`] (house style — the `Deref` is the
/// derive, never hand-written); construct it via [`new`](TerrainUuid::new) (wrap an
/// existing `Uuid`) or [`generate`](TerrainUuid::generate) (mint a fresh one).
///
/// `#[serde(transparent)]` round-trips it as the bare `Uuid` wire form (a string in
/// RON's human-readable encoding), and [`TypePath`] lets it ride a reflected asset
/// payload the same way the sibling def types do.
/// Implements [`Default`] (the NIL-UUID sentinel) so
/// [`Situation`](crate::situation::Situation) can use `#[serde(default)]` on its
/// [`default_floor`](crate::situation::Situation::default_floor) field — an omitted field
/// parses as the nil key, which the setup treats as "no authored floor piece; fall back to
/// `CombatTuning::move_costs.open`" (the GTW-491 successor to the legacy
/// [`TerrainName`](crate::terrain::piece::TerrainName) empty-string sentinel).
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize, TypePath,
)]
#[serde(transparent)]
pub struct TerrainUuid(Uuid);

impl TerrainUuid {
    /// Wrap an existing [`Uuid`] as a terrain key — used when the UUID is supplied
    /// (an authored definition's key, or a reconstructed one).
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// The **nil** terrain key — the [`Default`] sentinel (an all-zero UUID) signalling
    /// "no authored terrain piece". [`Situation::default_floor`](crate::situation::Situation)
    /// uses it as the GTW-491 successor to the legacy empty-string sentinel: a nil key skips
    /// registry floor resolution and falls back to `CombatTuning::move_costs.open`.
    #[must_use]
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Whether this key is the [`nil`](TerrainUuid::nil) sentinel — the `#[serde(default)]`
    /// for an omitted `default_floor` field (signals "no authored floor piece, use the
    /// `CombatTuning::move_costs.open` fallback").
    #[must_use]
    pub const fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    /// Mint a **fresh, random** terrain key (UUID v4) — used when authoring a new
    /// terrain definition that has no key yet.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// A **deterministic** terrain key derived from a legacy
    /// [`TerrainName`](crate::terrain::piece::TerrainName) string — the GTW-491 procgen SHIM
    /// bridge.
    ///
    /// The legacy procgen path ([`emit_level`](crate::procgen::emit_level)) still reads
    /// `TerrainName`-keyed prefab fragments and must populate the now-UUID-keyed
    /// [`Situation`](crate::situation::Situation) terrain references; this folds a name into a
    /// stable v8-shaped UUID via the same FNV-1a-64 hash the RNG-stream derivation uses, so the
    /// same name always yields the same key (the procgen determinism property is preserved).
    /// It does NOT match a migrated [`TerrainDef`](super::TerrainDef)'s authored key — the
    /// full procgen switch onto real UUID-keyed v2 prefabs is GTW-492 (T07b), which removes
    /// this shim.
    #[must_use]
    pub fn from_legacy_name(name: &str) -> Self {
        Self(Uuid::from_u128(fnv1a64_u128(name.as_bytes())))
    }
}

/// A 128-bit FNV-1a hash of `bytes` — two independent FNV-1a-64 passes (forward + reversed)
/// packed into a `u128`, used only by [`TerrainUuid::from_legacy_name`] /
/// [`ThemeUuid::from_legacy_theme`](crate::level::ThemeUuid::from_legacy_theme) to mint a
/// stable UUID from a legacy key string (the GTW-491 procgen shim). Deterministic and
/// dependency-free, mirroring the RNG-stream FNV-1a-64 derivation.
pub(crate) fn fnv1a64_u128(bytes: &[u8]) -> u128 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let hash_pass = |iter: &mut dyn Iterator<Item = &u8>| {
        let mut hash = FNV_OFFSET;
        for &b in iter {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    };
    let high = hash_pass(&mut bytes.iter());
    let low = hash_pass(&mut bytes.iter().rev());
    (u128::from(high) << 64) | u128::from(low)
}
