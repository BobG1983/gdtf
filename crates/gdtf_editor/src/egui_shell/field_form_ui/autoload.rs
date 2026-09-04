use gdtf_battle_sim::effects::fields::{FieldDefRegistry, FieldKey};

use crate::field_form::FieldDraft;

pub(crate) fn autoload_first_field(draft: &mut FieldDraft, registry: &FieldDefRegistry) {
    if !draft.autoload_pending() {
        return;
    }
    let mut keys: Vec<&FieldKey> = registry.keys().collect();
    keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    match keys
        .first()
        .and_then(|key| registry.def(key).map(|def| ((*key).clone(), def.clone())))
    {
        Some((key, def)) => draft.load_field(&key, &def),
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        effects::fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, ImmuneArmorTypes,
        },
        weapon::DamageType,
    };

    use super::autoload_first_field;
    use crate::field_form::FieldDraft;

    fn def(damage: u16) -> FieldDef {
        FieldDef::new(
            FieldDamage::new(damage),
            DamageType::Chem,
            ImmuneArmorTypes::default(),
            FieldDuration::Permanent,
        )
    }

    #[test]
    fn seeds_first_sorted_field_exactly_once() {
        let registry = FieldDefRegistry::new([
            (FieldKey::new("zeta_pool".to_owned()), def(1)),
            (FieldKey::new("alpha_pool".to_owned()), def(2)),
        ]);
        let mut draft = FieldDraft::default();
        autoload_first_field(&mut draft, &registry);
        assert_eq!(draft.key(), "alpha_pool", "sorted-first pick");

        draft.set_key("renamed".to_owned());
        autoload_first_field(&mut draft, &registry);
        assert_eq!(
            draft.key(),
            "renamed",
            "a settled draft is never re-seeded over",
        );

        let mut empty_seeded = FieldDraft::default();
        autoload_first_field(&mut empty_seeded, &FieldDefRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.key(), "");
    }
}
