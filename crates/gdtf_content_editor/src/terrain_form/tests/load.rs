use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    slab::SlabHp,
    terrain::{
        def::{
            BlocksPathingOverride, LeavesBehind, LosBlocking, TerrainDef, TerrainDefRegistry,
            TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid,
            TerrainView, TerrainViewArt, TerrainViews, owed_views_for,
        },
        entity::TerrainPieceKind,
        facing::TerrainFacing,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::{DamageType, HitType, WeaponName},
};

use super::support::key;
use crate::terrain_form::{
    FootfallChoice, TerrainDraft, TerrainKindChoice, draft_to_terrain_def, load_candidates,
};

// The tag every fixture def carries. `owed_views_for` reads neither this nor the draft's.
const DEF_TAG: TerrainTag = TerrainTag::BlocksVision;

// The tag the draft a def is loaded into already holds, so a dropped `replace_tags` shows.
const DRAFT_TAG: TerrainTag = TerrainTag::Indestructible;

// A Slab def under one display name, for the picker rows.
fn slab_named(low: u128, name: &str) -> TerrainDef {
    TerrainDef {
        key:            TerrainUuid::new(Uuid::from_u128(low)),
        display_name:   TerrainDisplayName::new(name.to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(10),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab { footfall: None },
        views:          TerrainViews::new(Vec::new()),
        tags:           Vec::new(),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

// Four defs, two of them under one display name, so the suffix rule has something to fix.
fn picker_registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new(
        [
            (0x0184_0bcd_1001, "Bulkhead"),
            (0x0184_0bcd_1002, "Deck"),
            (0x0184_0bcd_1003, "Drum"),
            (0x0184_0bcd_1004, "Drum"),
        ]
        .into_iter()
        .map(|(low, name)| {
            let def = slab_named(low, name);
            (def.key, def)
        }),
    )
}

// The label a row would carry if it named the display name and nothing else.
fn display_name_of(registry: &TerrainDefRegistry, uuid: &TerrainUuid) -> String {
    registry
        .def(uuid)
        .map_or_else(String::new, |def| (*def.display_name).clone())
}

#[test]
fn the_picker_rows_come_back_sorted_by_display_name_and_no_two_read_alike() {
    let registry = picker_registry();
    let rows = load_candidates(&registry);
    assert_eq!(rows.len(), 4, "every def in the registry gets its own row");

    let names: Vec<String> = rows
        .iter()
        .map(|(uuid, _)| display_name_of(&registry, uuid))
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(
        names, sorted,
        "the registry is a HashMap underneath, so the rows are ordered by display name before \
         the picker draws them",
    );

    let mut labels: Vec<String> = rows.iter().map(|(_, label)| label.clone()).collect();
    labels.sort();
    labels.dedup();
    assert_eq!(
        labels.len(),
        rows.len(),
        "no two rows may read alike, or an author picking one of two defs under a shared \
         display name cannot tell which row is which: {rows:?}",
    );

    for (uuid, label) in &rows {
        let name = display_name_of(&registry, uuid);
        if name == "Drum" {
            assert_eq!(
                *label,
                format!("Drum  [{}]", (**uuid)),
                "a display name two or more defs share carries that def's own key",
            );
        } else {
            assert_eq!(
                *label, name,
                "a display name only one def holds is that row's whole label",
            );
        }
    }
}

#[test]
fn loading_a_slab_fills_the_cover_hp_box_the_form_draws() {
    let mut def = slab_named(0x0184_0bcd_2001, "Deck Plate");
    def.sim_kind = TerrainSimKind::Slab {
        hp:               SlabHp::new(120),
        armor_protection: ArmorProtection::new(1),
        armor_hardness:   ArmorHardness::new(2),
    };

    let mut draft = TerrainDraft::default();
    assert_eq!(
        *draft.cover_hp(),
        40,
        "the case starts from the default cover HP, or the assertion below proves nothing",
    );
    draft.load_from_def(&def);

    assert_eq!(
        *draft.cover_hp(),
        120,
        "the Max HP box reads `cover_hp` for every kind, so loading a Slab must fill it too",
    );
}

#[test]
fn loading_a_wall_fills_the_slab_hp_a_kind_switch_would_read() {
    let mut def = slab_named(0x0184_0bcd_2002, "Bulkhead");
    def.sim_kind = TerrainSimKind::Wall {
        hp:               CoverHp::new(90),
        armor_protection: ArmorProtection::new(1),
        armor_hardness:   ArmorHardness::new(2),
        height_band:      HeightBand::Mid,
    };
    def.presenter_kind = TerrainPresenterKind::Wall;

    let mut draft = TerrainDraft::default();
    assert_eq!(
        *draft.slab_hp(),
        50,
        "the case starts from the default slab HP, or the assertion below proves nothing",
    );
    draft.load_from_def(&def);

    assert_eq!(
        *draft.slab_hp(),
        90,
        "switching the loaded def to Slab must carry the HP the author loaded, not the default",
    );
}

#[test]
fn a_load_takes_the_defs_own_key_and_the_next_save_mints_nothing() {
    let mut draft = TerrainDraft::default();
    let minted = draft.ensure_uuid();
    let mut def = slab_named(0x0184_0bcd_3001, "Keyed Probe");
    def.key = key();
    assert_ne!(
        minted, def.key,
        "the draft minted a key of its own first, so the load has one to overwrite",
    );

    draft.load_from_def(&def);

    assert_eq!(
        draft.uuid(),
        Some(def.key),
        "a load takes the def's authored key, so the next save writes that record",
    );
    assert_eq!(
        draft.ensure_uuid(),
        def.key,
        "the save button asks `ensure_uuid`, and after a load it must answer the loaded key \
         rather than minting a second one",
    );
}

// The sim side of one kind, every field a non-default value.
fn sim_kind_for(kind: TerrainPieceKind) -> TerrainSimKind {
    let armor_protection = ArmorProtection::new(17);
    let armor_hardness = ArmorHardness::new(19);
    match kind {
        TerrainPieceKind::Wall => TerrainSimKind::Wall {
            hp: CoverHp::new(123),
            armor_protection,
            armor_hardness,
            height_band: HeightBand::Mid,
        },
        TerrainPieceKind::Cover => TerrainSimKind::Cover {
            hp: CoverHp::new(123),
            armor_protection,
            armor_hardness,
            height_band: HeightBand::Mid,
        },
        TerrainPieceKind::Slab => TerrainSimKind::Slab {
            hp: SlabHp::new(211),
            armor_protection,
            armor_hardness,
        },
        TerrainPieceKind::Emplacement => TerrainSimKind::Emplacement {
            hp: CoverHp::new(123),
            armor_protection,
            armor_hardness,
            height_band: HeightBand::Mid,
            mounted_weapon: WeaponName::new("heavy_stubber".to_owned()),
            entry_sides: vec![TerrainFacing::South, TerrainFacing::West],
        },
    }
}

// The presenter side the projection builds from the draft's kind, so the fixture matches it.
fn presenter_kind_for(kind: TerrainPieceKind) -> TerrainPresenterKind {
    match kind {
        TerrainPieceKind::Wall => TerrainPresenterKind::Wall,
        TerrainPieceKind::Cover => TerrainPresenterKind::Cover,
        TerrainPieceKind::Slab => TerrainPresenterKind::Slab {
            footfall: Some(FootfallSound::new("footfall_metal".to_owned())),
        },
        TerrainPieceKind::Emplacement => TerrainPresenterKind::Emplacement,
    }
}

// One def per kind, carrying a non-default value in every field that kind's def holds.
fn filled_def(kind: TerrainPieceKind) -> TerrainDef {
    let tags = vec![DEF_TAG];
    let views = TerrainViews::new(
        owed_views_for(kind, &tags)
            .iter()
            .map(|view| TerrainViewArt {
                view:   *view,
                sprite: TerrainGraphicKey::new(format!("{view:?}").to_lowercase()),
            })
            .collect(),
    );
    TerrainDef {
        key: key(),
        display_name: TerrainDisplayName::new(format!("{kind:?} Identity Probe")),
        sim_kind: sim_kind_for(kind),
        presenter_kind: presenter_kind_for(kind),
        views,
        tags,
        on_death: vec![OnDeathEffect::Explode {
            hit_type:    HitType::Single,
            damage:      ExplodeDamage::new(37),
            damage_type: DamageType::Blast,
        }],
        blocks_pathing: Some(BlocksPathingOverride::new(true)),
        blocks_los: Some(LosBlocking::Full),
        leaves_behind: LeavesBehind::Sprite(TerrainGraphicKey::new("rubble".to_owned())),
    }
}

// A draft whose every field contrasts with the fixture, so a field the load skips shows up.
fn contrasting_draft() -> TerrainDraft {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Stale Draft".to_owned());
    draft.set_kind(TerrainKindChoice::Slab);
    draft.set_footfall(FootfallChoice::Grate);
    draft.set_cover_hp(CoverHp::new(7));
    draft.set_slab_hp(SlabHp::new(9));
    draft.set_armor_protection(ArmorProtection::new(11));
    draft.set_armor_hardness(ArmorHardness::new(13));
    draft.set_height_band(HeightBand::Low);
    draft.set_view(
        TerrainView::Single,
        TerrainGraphicKey::new("stale".to_owned()),
    );
    draft.replace_tags(vec![DRAFT_TAG]);
    draft.set_blocks_pathing(Some(false));
    draft.set_blocks_los(Some(LosBlocking::None));
    draft.set_leaves_behind(LeavesBehind::Piece(TerrainUuid::nil()));
    draft.replace_on_death(vec![OnDeathEffect::LeaveField {
        field: FieldKey::new("stale_field".to_owned()),
    }]);
    draft.set_uuid(Some(TerrainUuid::nil()));
    draft
}

#[test]
fn loading_a_def_of_any_kind_and_saving_it_again_writes_the_same_def() {
    for kind in TerrainPieceKind::ALL {
        let def = filled_def(kind);
        let mut loaded = contrasting_draft();
        loaded.load_from_def(&def);

        let Ok(projected) = draft_to_terrain_def(&loaded, def.key) else {
            unreachable!("a loaded {kind:?} draft must project back (the fixture names a weapon)")
        };
        assert_eq!(
            projected, def,
            "opening a {kind:?} def in the Terrain form and saving it again must write the def \
             it was opened from, field for field",
        );
    }
}
