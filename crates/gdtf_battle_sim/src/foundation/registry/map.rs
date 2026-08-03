//! Generic lookup map every authored-content registry builds on.

use std::hash::Hash;

use bevy::platform::collections::HashMap;

/// Keyed collection for sim content registries.
#[derive(Debug, Clone)]
pub struct Registry<K, V>(HashMap<K, V>);

impl<K: Eq + Hash, V> Registry<K, V> {
    /// Build from key–value pairs.
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (K, V)>) -> Self {
        Self(entries.into_iter().collect())
    }

    /// Insert or replace a value; returns the previous value if any.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// Borrow the value for `key`.
    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

    /// True if `key` is present.
    #[must_use]
    pub fn contains(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

    /// Iterate key–value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }
}

impl<K: Eq + Hash, V> Default for Registry<K, V> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl<K: Eq + Hash, V: PartialEq> PartialEq for Registry<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K: Eq + Hash, V: Eq> Eq for Registry<K, V> {}

impl<'a, K, V> IntoIterator for &'a Registry<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
