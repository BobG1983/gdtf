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

// The five content registries whose delete drops a record through `remove`.
mod wrappers {
    use crate::{
        armor::{ArmorName, ArmorRegistry, ArmorSpec, InjuryCategory},
        effects::fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, ImmuneArmorTypes,
        },
        equipment::attachments::{
            AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec,
        },
        injuries::{
            InjuryDef, InjuryName, InjuryRegistry, InspectText, LogText, PopupText, PostHeal,
        },
        severity::Severity,
        test_support::{arbitrary_armor, test_melee_weapon_spec},
        weapon::{DamageType, MeleeWeaponRegistry, MeleeWeaponSpec, WeaponName},
    };

    // The key kept, and the key removed, in every case below.
    const KEPT: &str = "kept";
    const DROPPED: &str = "dropped";

    fn armor_name(key: &str) -> ArmorName {
        ArmorName::new(key.to_owned())
    }

    fn weapon_name(key: &str) -> WeaponName {
        WeaponName::new(key.to_owned())
    }

    fn attachment_name(key: &str) -> AttachmentName {
        AttachmentName::new(key.to_owned())
    }

    fn injury_name(key: &str) -> InjuryName {
        InjuryName::new(key.to_owned())
    }

    fn field_key(key: &str) -> FieldKey {
        FieldKey::new(key.to_owned())
    }

    fn attachment_spec() -> AttachmentSpec {
        AttachmentSpec {
            display_name: weapon_name("probe sight"),
            slot:         AttachmentSlot::Sight,
            effects:      Vec::new(),
        }
    }

    fn injury_def(key: &str) -> InjuryDef {
        InjuryDef {
            name:         injury_name(key),
            category:     InjuryCategory::Head,
            severity:     Severity::Minor,
            popup_text:   PopupText::new("probe".to_owned()),
            log_text:     LogText::new("probe".to_owned()),
            inspect_text: InspectText::new("probe".to_owned()),
            effects:      Vec::new(),
            post_heal:    PostHeal::Deferred,
        }
    }

    fn field_def() -> FieldDef {
        FieldDef::new(
            FieldDamage::new(1),
            DamageType::Chem,
            ImmuneArmorTypes::new([]),
            FieldDuration::Permanent,
        )
    }

    #[test]
    fn armor_remove_drops_one_key_and_keeps_the_other() {
        let mut registry = ArmorRegistry::new([
            (armor_name(KEPT), arbitrary_armor(1)),
            (armor_name(DROPPED), arbitrary_armor(2)),
        ]);

        let taken: Option<ArmorSpec> = registry.remove(&armor_name(DROPPED));

        assert_eq!(taken, Some(arbitrary_armor(2)));
        assert_eq!(registry.spec(&armor_name(DROPPED)), None);
        assert_eq!(registry.spec(&armor_name(KEPT)), Some(&arbitrary_armor(1)));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn melee_weapon_remove_drops_one_key_and_keeps_the_other() {
        let mut registry = MeleeWeaponRegistry::new([
            (weapon_name(KEPT), test_melee_weapon_spec()),
            (weapon_name(DROPPED), test_melee_weapon_spec()),
        ]);

        let taken: Option<MeleeWeaponSpec> = registry.remove(&weapon_name(DROPPED));

        assert_eq!(taken, Some(test_melee_weapon_spec()));
        assert_eq!(registry.spec(&weapon_name(DROPPED)), None);
        assert!(registry.spec(&weapon_name(KEPT)).is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn attachment_remove_drops_one_key_and_keeps_the_other() {
        let mut registry = AttachmentRegistry::new([
            (attachment_name(KEPT), attachment_spec()),
            (attachment_name(DROPPED), attachment_spec()),
        ]);

        let taken: Option<AttachmentSpec> = registry.remove(&attachment_name(DROPPED));

        assert_eq!(taken, Some(attachment_spec()));
        assert_eq!(registry.spec(&attachment_name(DROPPED)), None);
        assert!(registry.spec(&attachment_name(KEPT)).is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn injury_remove_drops_one_key_and_keeps_the_other() {
        let mut registry = InjuryRegistry::new([
            (injury_name(KEPT), injury_def(KEPT)),
            (injury_name(DROPPED), injury_def(DROPPED)),
        ]);

        let taken: Option<InjuryDef> = registry.remove(&injury_name(DROPPED));

        assert_eq!(taken, Some(injury_def(DROPPED)));
        assert_eq!(registry.def(&injury_name(DROPPED)), None);
        assert_eq!(registry.def(&injury_name(KEPT)), Some(&injury_def(KEPT)));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn field_def_remove_drops_one_key_and_keeps_the_other() {
        let mut registry = FieldDefRegistry::new([
            (field_key(KEPT), field_def()),
            (field_key(DROPPED), field_def()),
        ]);

        let taken: Option<FieldDef> = registry.remove(&field_key(DROPPED));

        assert_eq!(taken, Some(field_def()));
        assert_eq!(registry.def(&field_key(DROPPED)), None);
        assert_eq!(registry.def(&field_key(KEPT)), Some(&field_def()));
        assert_eq!(registry.len(), 1);
    }
}
