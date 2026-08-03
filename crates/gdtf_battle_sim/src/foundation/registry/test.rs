use crate::registry::Registry;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProbeKey(&'static str);

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
    assert_eq!(registry.get(&ProbeKey("gamma")), None);
}

#[test]
fn insert_returns_previous_definition() {
    let mut registry = Registry::new([(ProbeKey("alpha"), ProbeDef(1))]);
    assert_eq!(registry.insert(ProbeKey("beta"), ProbeDef(2)), None);
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

    let keys: Vec<&ProbeKey> = registry.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&&ProbeKey("alpha")));
    assert!(keys.contains(&&ProbeKey("beta")));

    let pairs: Vec<(&ProbeKey, &ProbeDef)> = registry.iter().collect();
    assert_eq!(pairs.len(), 2);
    assert!(pairs.contains(&(&ProbeKey("alpha"), &ProbeDef(1))));
    assert!(pairs.contains(&(&ProbeKey("beta"), &ProbeDef(2))));

    let mut visited = 0;
    for (key, def) in &registry {
        assert_eq!(registry.get(key), Some(def));
        visited += 1;
    }
    assert_eq!(visited, 2);
}

#[test]
fn default_is_bound_free_over_defaultless_key_and_value() {
    let registry: Registry<ProbeKey, ProbeDef> = Registry::default();
    assert!(registry.is_empty());
}
