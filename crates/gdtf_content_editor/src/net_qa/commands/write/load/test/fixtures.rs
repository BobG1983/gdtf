use gdtf_battle_sim::{
    armor::{ArmorName, ArmorPiece, ArmorRegistry, ArmorSpec},
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec},
    ganger::{GangName, GangRegistry, GangRoster},
    injuries::{InjuryDef, InjuryName, InjuryRegistry},
    level::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry},
    terrain::def::TerrainUuid,
    weapon::{MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};
use gdtf_content_families::sprites::{
    SpriteAnchor, SpriteDef, SpriteDefRegistry, SpriteImagePath, SpriteName, SpritePx, SpriteSource,
};

use crate::{
    melee_weapon_form::MeleeWeaponDraft, net_qa::wire::EditorKeyNet, weapon_form::WeaponDraft,
};

/// The one key every fixture registry below holds.
pub(super) const SEEDED_KEY: &str = "alpha_seed";

/// A key as a client sends it over the wire.
pub(super) fn asked_key(key: &str) -> EditorKeyNet {
    EditorKeyNet::new(key.to_owned())
}

/// The seeded key, as a client sends it over the wire.
pub(super) fn seeded_key() -> EditorKeyNet {
    asked_key(SEEDED_KEY)
}

pub(super) fn gangs() -> GangRegistry {
    GangRegistry::new([(GangName::new(SEEDED_KEY.to_owned()), GangRoster::default())])
}

pub(super) fn armor() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(SEEDED_KEY.to_owned()),
        ArmorSpec::uniform(ArmorPiece::default()),
    )])
}

pub(super) fn injuries() -> InjuryRegistry {
    InjuryRegistry::new([(InjuryName::new(SEEDED_KEY.to_owned()), injury_def())])
}

pub(super) fn sprites() -> SpriteDefRegistry {
    SpriteDefRegistry::new([(SpriteName::new(SEEDED_KEY.to_owned()), sprite_def())])
}

pub(super) fn attachments() -> AttachmentRegistry {
    AttachmentRegistry::new([(
        AttachmentName::new(SEEDED_KEY.to_owned()),
        AttachmentSpec {
            display_name: WeaponName::new("Ash Optic".to_owned()),
            slot:         AttachmentSlot::Sight,
            effects:      Vec::new(),
        },
    )])
}

pub(super) fn weapons() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(SEEDED_KEY.to_owned()),
        WeaponDraft::new_weapon().spec().clone(),
    )])
}

pub(super) fn melee_weapons() -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([(
        WeaponName::new(SEEDED_KEY.to_owned()),
        MeleeWeaponDraft::new_melee_weapon().spec().clone(),
    )])
}

pub(super) fn theme_def() -> UuidThemeDef {
    UuidThemeDef {
        key:           ThemeUuid::generate(),
        display_name:  ThemeDisplayName::new("Ash Wastes".to_owned()),
        default_floor: TerrainUuid::nil(),
        terrain:       Vec::new(),
    }
}

pub(super) fn themes(def: &UuidThemeDef) -> UuidThemeRegistry {
    UuidThemeRegistry::new([(def.key, def.clone())])
}

fn sprite_def() -> SpriteDef {
    SpriteDef {
        source:    SpriteSource::File(SpriteImagePath::new("sprites/a.png".to_owned())),
        anchor:    SpriteAnchor {
            x: SpritePx::new(8),
            y: SpritePx::new(8),
        },
        facings:   None,
        animation: None,
    }
}

fn injury_def() -> InjuryDef {
    let ron = format!(
        "(name: \"{SEEDED_KEY}\", category: Leg, severity: Minor, popup_text: \"X\", \
         log_text: \"x\", inspect_text: \"x\", effects: [DisableHand])",
    );
    let parsed = ron::de::from_str::<InjuryDef>(&ron);
    assert!(parsed.is_ok(), "fixture injury def must parse: {parsed:?}");
    let Ok(def) = parsed else {
        unreachable!("asserted Ok above")
    };
    def
}
