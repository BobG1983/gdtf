//! Unit tests for the generic [`Registry`] catalog map (GTW-567) — stamped ONCE
//! here; the per-family wrappers are one-line delegations and do NOT re-test it.

use crate::registry::Registry;

/// A probe KEY deliberately WITHOUT `Default` (and without `Ord`) — mirrors the
/// real key newtypes (`ArmorName`, `FieldKey`) so the bound-free-`Default` test
/// below proves the hand-written impl, not a derive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProbeKey(&'static str);

/// A probe DEFINITION deliberately WITHOUT `Default`, mirroring the real spec/def
/// value types the families hold by value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeDef(u8);

#[test]
fn new_collects_and_get_hits_and_misses() {
    let registry = Registry::new([
        (ProbeKey("alpha"), ProbeDef(1)),
        (ProbeKey("beta"), ProbeDef(2)),
    ]);
    assert_eq!(registry.get(&ProbeKey("alpha")), Some(&ProbeDef(1)));
    assert_eq!(registry.get(&ProbeKey("beta")), Some(&ProbeDef(2)));
    // A key nothing was registered under misses.
    assert_eq!(registry.get(&ProbeKey("gamma")), None);
}

#[test]
fn insert_returns_previous_definition() {
    let mut registry = Registry::new([(ProbeKey("alpha"), ProbeDef(1))]);
    // A fresh key had no previous occupant.
    assert_eq!(registry.insert(ProbeKey("beta"), ProbeDef(2)), None);
    // Re-inserting an occupied key hands back the displaced definition.
    assert_eq!(
        registry.insert(ProbeKey("alpha"), ProbeDef(9)),
        Some(ProbeDef(1))
    );
    assert_eq!(registry.get(&ProbeKey("alpha")), Some(&ProbeDef(9)));
}

#[test]
fn contains_hits_and_misses() {
    let registry = Registry::new([(ProbeKey("alpha"), ProbeDef(1))]);
    assert!(registry.contains(&ProbeKey("alpha")));
    assert!(!registry.contains(&ProbeKey("gamma")));
}

#[test]
fn len_and_is_empty_track_contents() {
    let mut registry = Registry::new([]);
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    registry.insert(ProbeKey("alpha"), ProbeDef(1));
    registry.insert(ProbeKey("beta"), ProbeDef(2));
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 2);
}

#[test]
fn keys_iter_and_into_iterator_enumerate_every_entry() {
    let registry = Registry::new([
        (ProbeKey("alpha"), ProbeDef(1)),
        (ProbeKey("beta"), ProbeDef(2)),
    ]);

    // keys(): every key, no map exposure (order unspecified — assert membership).
    let keys: Vec<&ProbeKey> = registry.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&&ProbeKey("alpha")));
    assert!(keys.contains(&&ProbeKey("beta")));

    // iter(): every (key, definition) pair.
    let pairs: Vec<(&ProbeKey, &ProbeDef)> = registry.iter().collect();
    assert_eq!(pairs.len(), 2);
    assert!(pairs.contains(&(&ProbeKey("alpha"), &ProbeDef(1))));
    assert!(pairs.contains(&(&ProbeKey("beta"), &ProbeDef(2))));

    // IntoIterator for &Registry: the for-loop form the trait exists to allow.
    let mut visited = 0;
    for (key, def) in &registry {
        assert_eq!(registry.get(key), Some(def));
        visited += 1;
    }
    assert_eq!(visited, 2);
}

#[test]
fn default_is_bound_free_over_defaultless_key_and_value() {
    // ProbeKey/ProbeDef carry NO Default impl — this compiles ONLY because
    // Registry's Default is hand-written without K/V `Default` bounds.
    let registry: Registry<ProbeKey, ProbeDef> = Registry::default();
    assert!(registry.is_empty());
}
