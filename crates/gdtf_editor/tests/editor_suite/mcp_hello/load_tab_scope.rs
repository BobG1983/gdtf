use crate::{
    mcp_hello::{
        client::{
            EDITOR_LOAD_ARMOR, EDITOR_LOAD_ATTACHMENT, EDITOR_LOAD_FIELD, EDITOR_LOAD_GANG,
            EDITOR_LOAD_INJURY, EDITOR_LOAD_MELEE_WEAPON, EDITOR_LOAD_SPRITE, EDITOR_LOAD_TERRAIN,
            EDITOR_LOAD_THEME, EDITOR_LOAD_WEAPON,
        },
        lifecycle::open_tab,
    },
    mcp_shared::{
        harness::editing_app_and_client, outcome::unavailable_code, socket::run_editor,
        support::TestResult,
    },
};

/// A key no case reaches, because every one of them is refused before the registry is read.
const ANY_KEY: &str = "whatever_key_the_caller_asked_for";

// Open a form tab that is not the command's own, then send that command and read the refusal.
fn refused_off_its_own_tab(command: &'static str, other_tab: &str) -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, other_tab)?;

    let reply = client.exchange(
        &mut app,
        &run_editor(command, &format!("(key: \"{ANY_KEY}\")")),
    )?;

    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "`{command}` fills the draft of one tab, so on the {other_tab} tab it is refused for the \
         open tab rather than answering a Ran reply about a draft the author cannot see",
    );
    Ok(())
}

#[test]
fn load_theme_is_refused_off_the_theme_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_THEME, "Armor")
}

#[test]
fn load_gang_is_refused_off_the_gang_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_GANG, "Armor")
}

#[test]
fn load_armor_is_refused_off_the_armor_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_ARMOR, "Gang")
}

#[test]
fn load_injury_is_refused_off_the_injury_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_INJURY, "Armor")
}

#[test]
fn load_sprite_is_refused_off_the_sprite_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_SPRITE, "Armor")
}

#[test]
fn load_attachment_is_refused_off_the_attachment_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_ATTACHMENT, "Armor")
}

#[test]
fn load_weapon_is_refused_off_the_weapon_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_WEAPON, "Armor")
}

#[test]
fn load_melee_weapon_is_refused_off_the_melee_weapon_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_MELEE_WEAPON, "Armor")
}

#[test]
fn load_field_is_refused_off_the_field_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_FIELD, "Armor")
}

#[test]
fn load_terrain_is_refused_off_the_terrain_tab() -> TestResult {
    refused_off_its_own_tab(EDITOR_LOAD_TERRAIN, "Armor")
}
