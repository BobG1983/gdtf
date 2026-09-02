use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    terrain::{
        def::{LeavesBehind, TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind},
        piece::TerrainGraphicKey,
    },
    weapon::{DamageType, HitType},
};

use super::support::key;
use crate::terrain_form::{TerrainDraft, draft_to_terrain_def};

#[test]
fn save_preserves_every_authored_on_death_effect_in_order() {
    let explode = OnDeathEffect::Explode {
        hit_type:    HitType::Single,
        damage:      ExplodeDamage::new(40),
        damage_type: DamageType::Blast,
    };
    let leave = OnDeathEffect::LeaveField {
        field: FieldKey::new("incendiary_fire".to_owned()),
    };
    let def = TerrainDef {
        key:            key(),
        display_name:   TerrainDisplayName::new("Waste Drum".to_owned()),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(3),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags:           Vec::new(),
        on_death:       vec![explode.clone(), leave.clone()],
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    };

    let mut draft = TerrainDraft::default();
    draft.load_from_def(&def);
    let Ok(saved) = draft_to_terrain_def(&draft, def.key) else {
        unreachable!("a loaded Cover draft always projects")
    };
    assert_eq!(
        saved.on_death,
        vec![explode, leave],
        "loading a def and saving it must keep EVERY authored effect, in the authored order — \
         a save that drops all but the first shortens the list here",
    );
    assert_eq!(
        saved, def,
        "load then save is identity for every field the form owns, including on_death",
    );
}
