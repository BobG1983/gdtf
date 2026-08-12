//! EVERY ganger without an authored melee weapon falls back to, so the form must load
use gdtf_assets::{ContentFamily, workspace_assets_root};
use gdtf_battle_sim::weapon::{FISTS_KEY, MeleeWeaponRegistry, WeaponName};
use gdtf_content_editor::{MeleeWeaponDraft, draft_to_melee_weapon_spec, write_melee_weapon_in};
use gdtf_content_families::MeleeWeaponsFamily;

use crate::harness::{advance_to_editing, editor_app_with_asset_root};

fn fists_file_name() -> String {
    format!("{FISTS_KEY}.{}", MeleeWeaponsFamily::EXTENSION)
}

#[test]
fn shipped_fists_shape_round_trips_through_the_form_without_drift() {
    let Some(assets) = workspace_assets_root() else {
        unreachable!("this repo has a Cargo.lock above every crate");
    };
    let shipped = assets
        .join(MeleeWeaponsFamily::FOLDER)
        .join(fists_file_name());
    let bytes = std::fs::read(&shipped);
    assert!(
        bytes.is_ok(),
        "the shipped fists file must exist at {}: {:?}",
        shipped.display(),
        bytes.as_ref().err(),
    );
    let Ok(bytes) = bytes else { return };
    let dir_a = tempfile::tempdir();
    assert!(dir_a.is_ok(), "creating TempDir root A must succeed");
    let Ok(dir_a) = dir_a else { return };
    let copy_dir = dir_a.path().join(MeleeWeaponsFamily::FOLDER);
    assert!(std::fs::create_dir_all(&copy_dir).is_ok());
    assert!(std::fs::write(copy_dir.join(fists_file_name()), bytes).is_ok());

    let mut app = editor_app_with_asset_root(dir_a.path());
    advance_to_editing(&mut app);
    let registry = app.world().get_resource::<MeleeWeaponRegistry>();
    assert!(registry.is_some(), "the MeleeWeaponRegistry must resolve");
    let Some(registry) = registry else { return };
    let loaded = registry.fists().cloned();
    assert!(
        loaded.is_some(),
        "the copied fists file must key FISTS_KEY through the real folder walk",
    );
    let Some(loaded) = loaded else { return };

    let mut draft = MeleeWeaponDraft::default();
    draft.load_melee_weapon(&WeaponName::new(FISTS_KEY.to_owned()), &loaded);
    let (name, resaved) = draft_to_melee_weapon_spec(&draft);
    assert_eq!(name.as_str(), FISTS_KEY, "the fists KEY survives the form");
    assert_eq!(resaved, loaded, "the form projection must not drift fists");

    let dir_b = tempfile::tempdir();
    assert!(dir_b.is_ok(), "creating TempDir root B must succeed");
    let Ok(dir_b) = dir_b else { return };
    let written = write_melee_weapon_in(dir_b.path(), &name, &resaved);
    assert!(
        written.is_ok(),
        "the real fists re-save must succeed: {:?}",
        written.as_ref().err(),
    );
    let mut app_b = editor_app_with_asset_root(dir_b.path());
    advance_to_editing(&mut app_b);
    let registry_b = app_b.world().get_resource::<MeleeWeaponRegistry>();
    assert!(registry_b.is_some(), "the re-save registry must resolve");
    let Some(registry_b) = registry_b else { return };
    assert_eq!(
        registry_b.fists(),
        Some(&loaded),
        "the re-saved fists must reload SEMANTICALLY IDENTICAL to the shipped shape \
         (still keyed FISTS_KEY — every ganger's unarmed fallback is intact)",
    );
}
