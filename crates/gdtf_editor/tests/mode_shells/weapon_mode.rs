//! Weapon mode: maximal-spec authoring round-trips through save and reload.
use std::{num::NonZeroU8, path::Path};

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::advance_until;
use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    equipment::attachments::{
        AttachmentName, AttachmentSlot, FittedAttachments, SlotCapacity, WeaponSlots,
    },
    weapon::{
        Accuracy, AmmoType, AoeRange, BaseSpread, BlastRadius, ConeHalfAngle, DamageType,
        DotDamage, DotProfile, DotTurns, FatalBias, FireMode, FireModeSpec, Handedness, HitType,
        Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Shove, Stable,
        TrajectoryStyle, WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred,
    },
};
use gdtf_editor::{
    EditorState, MapEditorPlugin, WeaponDraft, draft_to_weapon_spec, write_weapon_in,
};

fn editor_app_with_asset_root(root: &Path) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            }),
    );
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

fn maximal_draft() -> WeaponDraft {
    let mut draft = WeaponDraft::new_weapon();
    draft.set_name("tempdir_ripper_cannon".to_owned());
    let spec = draft.spec_mut();
    spec.base_spread = BaseSpread::new(0.22);
    spec.accuracy = Accuracy::new(1.3);
    spec.kickback = Kickback::new(0.07);
    spec.fatal_bias = FatalBias::new(0.4);
    spec.damage = WeaponDamage::new(11);
    spec.punch = WeaponPunch::new(4);
    spec.shred = WeaponShred::new(2);
    spec.damage_type = DamageType::Plasma;
    spec.accepts = AmmoType::Cell;
    spec.magazine.set_size(MagazineSize::new(18));
    spec.magazine
        .set_reload_tu(gdtf_battle_sim::magazine::ReloadTu::new(9));
    spec.fire_mode = FireMode::new(vec![
        FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.25),
            ModeShots::new(1),
        ),
        FireModeSpec::with_hit_type(
            ModeKind::Burst,
            ModeConeMult::new(1.35),
            ModeTuPercent::new(0.45),
            ModeShots::new(3),
            HitType::Blast {
                radius: BlastRadius::new(2),
            },
        ),
        FireModeSpec::with_hit_type(
            ModeKind::Full,
            ModeConeMult::new(1.8),
            ModeTuPercent::new(0.6),
            ModeShots::new(7),
            HitType::Cone {
                range: AoeRange::new(4),
                angle: ConeHalfAngle::new(22.5),
            },
        ),
    ]);
    spec.stable = Stable::new(true);
    spec.shove = Shove::new(true);
    spec.handedness = Handedness::TwoHanded;
    spec.trajectory = TrajectoryStyle::Arc;
    spec.slots = WeaponSlots::new(vec![
        (AttachmentSlot::Muzzle, SlotCapacity::new(1)),
        (AttachmentSlot::Rail, SlotCapacity::new(3)),
    ]);
    spec.attachments = FittedAttachments::new(vec![
        AttachmentName::new("suppressor".to_owned()),
        AttachmentName::new("scoped_sight".to_owned()),
    ]);
    spec.dot = Some(DotProfile::new(
        DotDamage::new(3),
        DamageType::Chem,
        DotTurns::new(NonZeroU8::MIN.saturating_add(3)),
    ));
    spec.on_death = vec![OnDeathEffect::Explode {
        hit_type:    HitType::Blast {
            radius: BlastRadius::new(2),
        },
        damage:      ExplodeDamage::new(6),
        damage_type: DamageType::Blast,
    }];
    draft
}

fn minimal_draft() -> WeaponDraft {
    let mut draft = WeaponDraft::new_weapon();
    draft.set_name("tempdir_scrap_tube".to_owned());
    let spec = draft.spec_mut();
    spec.base_spread = BaseSpread::new(0.1);
    spec.accuracy = Accuracy::new(0.9);
    spec.damage = WeaponDamage::new(5);
    spec.damage_type = DamageType::Kinetic;
    spec.magazine.set_size(MagazineSize::new(6));
    spec.magazine
        .set_reload_tu(gdtf_battle_sim::magazine::ReloadTu::new(8));
    draft
}

#[test]
fn leave_field_on_death_round_trips_through_ron() {
    let mut draft = WeaponDraft::new_weapon();
    draft.set_name("tempdir_barrel_bomb".to_owned());
    let authored = vec![
        OnDeathEffect::LeaveField {
            field: FieldKey::new("burning_ground".to_owned()),
        },
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(8),
            damage_type: DamageType::Blast,
        },
    ];
    draft.spec_mut().on_death = authored.clone();
    let (_, spec) = draft_to_weapon_spec(&draft);
    let serialized = ron::ser::to_string(&spec);
    assert!(serialized.is_ok(), "the spec serializes: {serialized:?}");
    let Ok(serialized) = serialized else { return };
    let parsed = ron::de::from_str::<gdtf_battle_sim::weapon::WeaponSpec>(&serialized);
    assert!(parsed.is_ok(), "the serialized spec parses: {parsed:?}");
    let Ok(parsed) = parsed else { return };
    assert_eq!(
        parsed.on_death, authored,
        "both authored effects survive the save, in the authored order",
    );
    assert_eq!(parsed, spec, "the on-death list survives verbatim");
}

#[test]
fn saved_weapons_round_trip_through_the_real_weapons_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let (max_name, max_spec) = draft_to_weapon_spec(&maximal_draft());
    let written = write_weapon_in(dir.path(), &max_name, &max_spec);
    assert!(
        written.is_ok(),
        "the real maximal weapon write must succeed: {:?}",
        written.as_ref().err(),
    );
    let (min_name, min_spec) = draft_to_weapon_spec(&minimal_draft());
    let written = write_weapon_in(dir.path(), &min_name, &min_spec);
    assert!(
        written.is_ok(),
        "the real minimal weapon write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<WeaponDraft>().is_some(),
        "the WeaponDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<WeaponRegistry>();
    assert!(registry.is_some(), "the WeaponRegistry must resolve");
    let Some(registry) = registry else { return };
    assert_eq!(
        registry.spec(&WeaponName::new("tempdir_ripper_cannon".to_owned())),
        Some(&max_spec),
        "the reloaded MAXIMAL weapon must equal the saved spec field-for-field (the \
         three fire modes incl. Blast/Cone payloads, the (Muzzle,1)+(Rail,3) slots, \
         both attachment keys, the Chem dot, the Explode on-death, and every scalar) — \
         the stem-key round-trip through the REAL loader",
    );
    assert_eq!(
        registry.spec(&WeaponName::new("tempdir_scrap_tube".to_owned())),
        Some(&min_spec),
        "the reloaded MINIMAL weapon must equal the saved spec — the serde-default \
         identities (shove off / Straight / no slots / no attachments / no dot / no \
         on_death) survive the serialize → parse chain",
    );
}
