//! The generic [`Registry`] catalog map (GTW-567): the shared `key → definition`
//! lookup surface, written once instead of hand-stamped per registry family.

use std::hash::Hash;

use bevy::platform::collections::HashMap;

/// The generic **catalog registry** — a `key → definition` map carrying the shared
/// lookup surface every authored-content registry family delegates to (GTW-567).
///
/// Nine registries (ranged/melee weapons, armor, attachments, injuries, field defs,
/// terrain defs, themes, gangs) each hand-stamped this same name→def newtype as a
/// self-described mirror of the last one, and the copies drifted (`contains` existed
/// only on injuries; enumeration was `defs()` on terrain/themes but `keys()`/`iter()`
/// on equipment). This type is that map written ONCE — the UNION of what the clones
/// re-implemented — so every family levels up to the full surface by delegation.
///
/// A named foundation type over a [`HashMap`] with a PRIVATE inner (no-bare-types
/// rule 5: this wrapper is the only place the raw map is touched). It is deliberately
/// **NOT** a [`Resource`](bevy::prelude::Resource) and NOT domain-facing: each family
/// keeps its own named `#[derive(Resource)]` newtype — its Bevy resource identity,
/// change-detection granularity, derive set, and domain-named lookups
/// (`spec`/`def`/`roster`) — holding a `Registry<K, V>` inner and delegating one-line
/// wrappers to it. Definitions are held BY VALUE, so they survive the loaded-folder
/// asset handle being dropped after `Load`.
///
/// Iteration order ([`keys`](Registry::keys) / [`iter`](Registry::iter)) is
/// unspecified — hash-map backed; callers that need a stable, reproducible order
/// (e.g. a sorted editor dropdown) must collect and sort.
#[derive(Debug, Clone)]
pub struct Registry<K, V>(HashMap<K, V>);

impl<K: Eq + Hash, V> Registry<K, V> {
    /// Build a registry from a `(key, definition)` iterator — the shape the folder
    /// loaders (and tests) build, keyed by each authored file's stem (or UUID).
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (K, V)>) -> Self {
        Self(entries.into_iter().collect())
    }

    /// Insert one definition under its key, returning the previous definition at
    /// that key (if any) — the per-file insert the folder loaders call as they
    /// iterate a loaded folder.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// Look up the definition for a key, or [`None`] if nothing is registered under
    /// it — the resolution the family's domain-named lookup (`spec`/`def`/`roster`)
    /// delegates to.
    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

    /// Whether the registry holds a definition under this key — the
    /// existence-audit question (e.g. does every injury-weighting entry resolve?).
    #[must_use]
    pub fn contains(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }

    /// How many definitions the registry holds — the count the folder-load tests
    /// assert.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no definitions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over every key — for editor dropdown / roster enumeration without
    /// exposing the inner map. Order is unspecified (see the type doc).
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

    /// Iterate over every `(key, definition)` pair — for enumeration that needs the
    /// definition alongside its key. Order is unspecified (see the type doc).
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }
}

/// Hand-written BOUND-FREE [`Default`] (an empty catalog): a DERIVED `Default`
/// would bound `K: Default + V: Default`, which key newtypes (e.g. `ArmorName`,
/// `FieldKey`) deliberately lack — a "default name" is meaningless.
impl<K: Eq + Hash, V> Default for Registry<K, V> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

/// Hand-written (not derived) so the bounds match the inner [`HashMap`]'s own
/// `PartialEq` impl (`K: Eq + Hash`, `V: PartialEq`) — a derive would bound only
/// `K: PartialEq`, which cannot compare a hash map.
impl<K: Eq + Hash, V: PartialEq> PartialEq for Registry<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// [`Eq`] exactly where both key and definition are `Eq` — so an `Eq` family
/// (armor / field defs / themes) can keep deriving `Eq` on its wrapper, while a
/// `PartialEq`-only family (f32-carrying defs) simply never instantiates this.
impl<K: Eq + Hash, V: Eq> Eq for Registry<K, V> {}

impl<'a, K, V> IntoIterator for &'a Registry<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, K, V>;

    /// Iterate over `(key, definition)` pairs via the [`IntoIterator`] trait —
    /// satisfies the `iter_without_into_iter` pedantic lint ONCE, for every family
    /// (previously re-stamped per registry). Delegates to the inner [`HashMap`]'s
    /// borrowed iterator; order is unspecified.
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
