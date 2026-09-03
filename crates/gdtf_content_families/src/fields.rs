use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
use gdtf_battle_sim::effects::fields::{FieldDef, FieldDefRegistry, FieldKey};

/// situation's authored `fields:` placements against.
pub struct FieldsFamily;

impl ContentFamily for FieldsFamily {
    type Spec = FieldDef;
    type Registry = FieldDefRegistry;

    const EXTENSION: &'static str = "field.ron";
    const FOLDER: &'static str = "content/fields";

    fn insert_member(
        registry: &mut FieldDefRegistry,
        stem: Option<ContentFileStem>,
        def: &FieldDef,
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.insert(FieldKey::new(key.clone()), def.clone());
        Some(ContentMemberKey::new(key))
    }
}
