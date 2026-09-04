//! C5/A2: the ATTACHMENT mode's REAL round-trip — author an attachment item
use std::path::Path;

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
use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSlot},
    weapon::{
        AoeRange, ConeHalfAngle, FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, WeaponName,
    },
};
use gdtf_editor::{
    AttachmentDraft, EditorState, MapEditorPlugin, draft_to_attachment_spec, write_attachment_in,
};
use gdtf_test_utils::advance_until;

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

fn edited_draft() -> AttachmentDraft {
    let mut draft = AttachmentDraft::new_attachment();
    draft.set_name("tempdir_conversion_kit".to_owned());
    let spec = draft.spec_mut();
    spec.display_name = WeaponName::new("Tempdir Conversion Kit".to_owned());
    spec.slot = AttachmentSlot::Rail;
    spec.effects = vec![
        AttachmentEffect::GainFireMode(FireModeSpec::with_hit_type(
            ModeKind::Burst,
            ModeConeMult::new(1.4),
            ModeTuPercent::new(0.35),
            ModeShots::new(3),
            HitType::Cone {
                range: AoeRange::new(4),
                angle: ConeHalfAngle::new(25.0),
            },
        )),
        AttachmentEffect::Silence,
        AttachmentEffect::Aim(AimDelta::new(0.25)),
    ];
    draft
}

#[test]
fn saved_attachment_round_trips_through_the_real_attachments_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let draft = edited_draft();
    let (name, spec) = draft_to_attachment_spec(&draft);
    let written = write_attachment_in(dir.path(), &name, &spec);
    assert!(
        written.is_ok(),
        "the real attachment write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<AttachmentDraft>().is_some(),
        "the AttachmentDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<AttachmentRegistry>();
    assert!(registry.is_some(), "the AttachmentRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.spec(&AttachmentName::new("tempdir_conversion_kit".to_owned()));
    assert_eq!(
        reloaded,
        Some(&spec),
        "the reloaded attachment must equal the saved spec (display name / Rail slot / \
         the GainFireMode burst-cone payload / Silence / the Aim magnitude) — the \
          stem-key round-trip through the REAL loader",
    );
}
