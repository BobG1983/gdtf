//! GTW-669 C5/A2: the ATTACHMENT mode's REAL round-trip — author an attachment item
//! (a `GainFireMode` effect with a full nested [`FireModeSpec`] payload PLUS a
//! payload-less effect, per the contract) in the form model, save it through the REAL
//! root-parameterized write (`write_attachment_in`) into a `TempDir` assets root (the
//! GTW-555 pattern — the shipped `assets/` tree is NEVER written), then boot the REAL
//! editor app rooted at that directory and assert the actual `AttachmentsFamily` folder
//! walk loads the saved item back structurally identical.
//!
//! Also pins the GTW-669 lifecycle riders: the editor reaches `Editing` with the
//! `AttachmentRegistry` gate resource present and the state-scoped `AttachmentDraft`
//! seeded (salvage / fallback behavior itself is the seam's parameterized family
//! contract — `register_content_family::<AttachmentsFamily>` inherits it, no per-family
//! re-pin here). The editor's weapon→attachment VALIDATION wiring (the registered
//! `check_weapon_attachment_refs` + the `AttachmentRegistry` watch-set re-arm,
//! GTW-669 C4) is pinned by its own suite: `tests/authoring_validation/attachments.rs`.

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
use gdtf_content_editor::{
    AttachmentDraft, EditorState, MapEditorPlugin, draft_to_attachment_spec, write_attachment_in,
};
use gdtf_test_utils::advance_until;

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) —
/// the test polls the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// The real editor app rooted at an ARBITRARY assets directory (the `sprite_mode.rs`
/// recipe): only `content/attachments/` is materialized by this test, so every other
/// family fails closed to its empty registry (the no-strand guarantee) while the
/// attachments walk loads the REAL saved file.
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
            // Headless-test noise suppression (GTW-139): the deliberate
            // failure-path asset errors of the unmaterialized families stay quiet.
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
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); with no render backend some render-provided params cannot
    // validate. `warn` restores the skip-with-a-log behavior (the shared harness
    // precedent).
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drives the app until [`EditorState::Editing`], then a few settle frames so the
/// `OnEnter(Editing)` command flushes apply before the assertions read.
fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — the Load gate (including the \
         AttachmentRegistry) did not resolve or fall back",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// The edited item the test authors through the REAL form-model mutators: the contract's
/// exact effect pair — a `GainFireMode` with a fully-authored nested [`FireModeSpec`]
/// (non-default kind / numbers / a payload-carrying `hit_type`) plus the payload-less
/// `Silence` — and a third payload-carrying effect so a dropped magnitude cannot
/// round-trip.
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

/// GTW-669 — create → save (the REAL write into a `TempDir` assets root) → load through
/// the REAL `AttachmentsFamily` folder walk → the registry holds the SAME spec
/// (structural equality across display name / slot / the `GainFireMode` + payload-less
/// effects, per the C5 contract), with the `Editing` gate + the scoped `AttachmentDraft`
/// seed along for the ride.
#[test]
fn saved_attachment_round_trips_through_the_real_attachments_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // SAVE through the real root-parameterized write.
    let draft = edited_draft();
    let (name, spec) = draft_to_attachment_spec(&draft);
    let written = write_attachment_in(dir.path(), &name, &spec);
    assert!(
        written.is_ok(),
        "the real attachment write must succeed: {:?}",
        written.as_ref().err(),
    );

    // RELOAD through the real editor Load pass rooted at the TempDir.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    // The state-scoped ATTACHMENT draft seeded on entering Editing (bevy-traps #1 via
    // the GTW-575 seam).
    assert!(
        world.get_resource::<AttachmentDraft>().is_some(),
        "the AttachmentDraft must be seeded OnEnter(Editing)",
    );

    // The REAL folder walk keyed the saved file by its stem and loaded the SAME spec.
    let registry = world.get_resource::<AttachmentRegistry>();
    assert!(registry.is_some(), "the AttachmentRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.spec(&AttachmentName::new("tempdir_conversion_kit".to_owned()));
    assert_eq!(
        reloaded,
        Some(&spec),
        "the reloaded attachment must equal the saved spec (display name / Rail slot / \
         the GainFireMode burst-cone payload / Silence / the Aim magnitude) — the \
         GTW-549 stem-key round-trip through the REAL loader",
    );
}
