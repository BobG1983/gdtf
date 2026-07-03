//! The area-damage-fields content family (GTW-545, generic seam since GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::{FieldDef, FieldDefRegistry, FieldKey};

/// The area-damage-fields family: `assets/content/fields/*.field.ron` → the
/// stem-keyed [`FieldDefRegistry`] catalog the battle setup resolves a
/// situation's authored `fields:` placements against.
///
/// STEM-KEYED: `toxic_waste_pool.field.ron` keys `toxic_waste_pool` (the
/// [`FieldKey`] a placement references).
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
        // Stem-keyed: a handle with no resolvable path/stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = stem else { return };
        registry.insert(FieldKey::new(stem.into_inner()), def.clone());
    }
}
