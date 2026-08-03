use gdtf_assets::{ContentFamily, ContentFileStem};
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
    ) {
        let Some(stem) = stem else { return };
        registry.insert(FieldKey::new(stem.into_inner()), def.clone());
    }
}
