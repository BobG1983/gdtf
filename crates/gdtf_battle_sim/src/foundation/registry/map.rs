use std::hash::Hash;

use bevy::platform::collections::HashMap;

/// lookup surface every authored-content registry family delegates to (GTW-567).
/// keeps its own named `#[derive(Resource)]` newtype — its Bevy resource identity,
#[derive(Debug, Clone)]
pub struct Registry<K, V>(HashMap<K, V>);

impl<K: Eq + Hash, V> Registry<K, V> {
            #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (K, V)>) -> Self {
        Self(entries.into_iter().collect())
    }

                pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

                #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

            #[must_use]
    pub fn contains(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }

            #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

            pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

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
