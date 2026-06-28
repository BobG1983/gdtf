//! Spawns the gang-editor screen on `OnEnter(RunningState::DebugEditor)` (GTW-420 scaffold +
//! GTW-425 collapsed rows + GTW-428 expanded stat table).
//!
//! Builds a themed panel layout holding:
//!
//! - a **title** heading,
//! - the gang-NAME text field via [`spawn_text_field`](gdtf_ui::spawn_text_field) (carrying the
//!   [`GangNameField`] marker so its commit maps to the model name — AC3),
//! - an **"Add member"** button via [`spawn_button`](gdtf_ui::spawn_button) (the
//!   [`AddMemberButton`] marker — AC4),
//! - the member-list SHELL: a GTW-412 [`ScrollList`](gdtf_ui::ScrollList) ([`MemberListHost`] on
//!   its root frame), seeded with one COLLAPSED [`MemberRow`] per member already in the model.
//!
//! Each member ROW is a flex column: a collapsed HEADER row above the GTW-428 expandable
//! [`MemberStatPanel`]. The header row (GTW-425 C1) contains, left→right: an [`ExpandPip`] `+`/`-`
//! toggle, a [`MemberPortrait`] placeholder, the inline [`MemberNameField`] beside a
//! [`MemberNameText`] echo, a [`MemberWeaponText`] + a [`MemberWeaponDropdown`] over all loaded
//! [`WeaponName`](gdtf_battle_sim::WeaponName) keys, a [`MemberArmorText`] + a
//! [`MemberArmorDropdown`] over all loaded [`ArmorName`](gdtf_battle_sim::ArmorName) keys, and a
//! [`DeleteMemberButton`]. BELOW it the GTW-428 stat panel lerps open on a pip press to show the
//! eight editable [`AttributeField`] numeric fields and the eight readonly [`DerivedStatText`]
//! displays in TWO COLUMNS — the panel animates open to a per-instance height waypoint then settles
//! to a CONTENT-FIT ([`Val::Auto`](bevy::ui::Val)) height (the GTW-428 layout fix, opting the panel
//! into [`AccordionContentFit`](gdtf_ui::AccordionContentFit)) so every line is visible regardless
//! of font metrics rather than clipped at the shared 18vh accordion default.
//!
//! Every entity carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit)
//! so the whole screen tears down on leave (C1). The model resource is inserted by a sibling
//! system ([`insert_editable_gang`](super::model_lifecycle::insert_editable_gang)) ordered before
//! this one, so the member count is known when the shell is seeded.

use bevy::{
    prelude::*,
    scene::CommandsSceneExt,
    ui::{Overflow, Val},
};
use gdtf_battle_sim::{
    ArmorName, ArmorRegistry, DerivedStats, GangerStatTuning, WeaponName, WeaponRegistry,
    derive_stats,
};
use gdtf_ui::{
    AccordionAnim, AccordionContent, AccordionContentFit, AccordionExpandedVh, AccordionProgress,
    ButtonLabel, CommittedTextValue, DropdownColors, DropdownOption, FieldColors, NumericRange,
    ScrollListColors, spawn_button, spawn_dropdown, spawn_numeric_field, spawn_panel,
    spawn_scroll_list, spawn_text_field,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    RunningState,
    running::editor::{
        components::{
            AddMemberButton, AttributeField, BaseAttribute, DeleteMemberButton, DerivedStat,
            DerivedStatText, EditorScreenRoot, ExpandPip, GangNameField, MemberArmorDropdown,
            MemberArmorText, MemberListHost, MemberNameField, MemberNameText, MemberPortrait,
            MemberRow, MemberRowIndex, MemberRowRef, MemberStatPanel, MemberWeaponDropdown,
            MemberWeaponText, PipExpanded,
        },
        model::{EditableGang, EditableMember},
        systems::derived_display::format_derived,
    },
};

/// Inter-child vertical gap of the editor column, in `Vh` — relative units (the responsive-UI
/// rule), reusing the menu's calibrated `10px / 720` separation so the editor spacing matches.
const EDITOR_GAP_VH: f32 = 1.388_89;

/// The collapsed `+` glyph the pip shows while a row is collapsed (the default — GTW-425 renders
/// the toggle state only; the expanded panel is GTW-428).
const PIP_COLLAPSED_GLYPH: &str = "+";

/// The portrait placeholder's side length, in viewport-width units — a small square colored block
/// standing in for the (not-yet-built) portrait system (C1). Relative, per the responsive-UI rule.
const PORTRAIT_SIDE_VW: f32 = 2.0;

/// The inclusive lower bound an editable base-attribute numeric field clamps into (GTW-428 C2).
/// Attributes are non-negative dimensionless magnitudes (`docs/combat/stats.md`), so the floor is
/// zero — a negative attribute is meaningless.
const ATTRIBUTE_MIN: f32 = 0.0;

/// The inclusive upper bound an editable base-attribute numeric field clamps into (GTW-428 C2). A
/// generous ceiling well above any authored attribute (the shipped gangers sit in low single
/// digits), so the editor never refuses a plausible value yet still clamps absurd input.
const ATTRIBUTE_MAX: f32 = 100.0;

/// The inter-field vertical gap inside the expanded stat panel, in `Vh` — reuses the editor's
/// calibrated row gap so the panel spacing matches the rest of the screen. Relative units.
const STAT_PANEL_GAP_VH: f32 = EDITOR_GAP_VH;

/// The `Vh` height the GTW-428 member stat panel's open ANIMATION lerps up to before it settles
/// (the GTW-428 layout fix). This is only the animation waypoint, NOT the final rest height: the
/// panel also carries [`AccordionContentFit`], so once the lerp settles fully open the shared
/// `drive_accordions` switches the height to [`Val::Auto`](bevy::ui::Val::Auto) and the rest-open
/// panel sizes to its EXACT content — every one of the sixteen stat lines renders regardless of the
/// rendered font / padding (a fixed `Vh` ceiling could not reliably clear the taller
/// eight-attribute-field column at the live window size — the round-2 QA defect). A generous-but-
/// sub-viewport waypoint so the open reads as a clear animation; the now-taller rest-open row
/// scrolls within the member-list scroll area, so one open member never crowds the others off.
/// Relative units (the responsive-UI rule); fed to the generalized accordion via
/// [`AccordionExpandedVh`].
const STAT_PANEL_EXPANDED_VH: f32 = 38.0;

/// The clamp range every editable base-attribute numeric field uses (`[ATTRIBUTE_MIN,
/// ATTRIBUTE_MAX]`) — built once and shared by every attribute field a row spawns (C2).
const fn attribute_range() -> NumericRange<f32> {
    NumericRange::new(ATTRIBUTE_MIN, ATTRIBUTE_MAX)
}

/// Spawns the full editor screen when [`RunningState::DebugEditor`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1) — in the running app the theme is present by the time the menu can
/// reach the editor. Reads the [`EditableGang`] model (inserted by the ordered-before sibling
/// system), the [`WeaponRegistry`] / [`ArmorRegistry`] (global resources the `Load` flow
/// inserts), and the [`GangerStatTuning`] (the GTW-384 derivation weights, a persistent resource
/// the `Load` flow inserts) as `Option<Res<…>>` so it is robust if any is absent — an absent
/// tuning seeds the readonly derived displays with the const-default derivation (C3). Each spawned
/// node carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit).
pub(in crate::states::running::editor) fn spawn_editor_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    model: Option<Res<EditableGang>>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    tuning: Option<Res<GangerStatTuning>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed screen (the menu precedent).
        return;
    };

    // ROOT — a centered, full-viewport themed backdrop column.
    let root = commands
        .spawn((
            EditorScreenRoot,
            Themed::new(ThemeRole::Background),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Vh(EDITOR_GAP_VH),
                ..default()
            },
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // TITLE — a themed heading, a direct child of the root.
    let title = commands
        .spawn_scene((
            bevy::scene::bsn! {
                Themed::new(ThemeRole::Title)
                Text::new("GANG EDITOR")
            },
            bevy::scene::template_value(DespawnOnExit(RunningState::DebugEditor)),
        ))
        .id();

    // PANEL — wraps the editing controls in a themed box, a flex column.
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        DespawnOnExit(RunningState::DebugEditor),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(EDITOR_GAP_VH),
            // A flex item that hosts a scroll list must allow itself to shrink below its content
            // (CSS `min-height: 0`) or the list bottom-cramps off-screen (the GTW-421 trap).
            min_height: Val::Px(0.0),
            ..default()
        },
    ));

    // GANG-NAME field — seeded with the model's current name, carrying the GangNameField marker
    // so the commit listener maps it to the model name (AC3).
    let initial_name = model
        .as_ref()
        .map(|model| model.name().as_str().to_owned())
        .unwrap_or_default();
    let name_field = spawn_text_field(
        &mut commands,
        CommittedTextValue::new(initial_name),
        field_colors(&theme),
        (GangNameField, DespawnOnExit(RunningState::DebugEditor)),
    );

    // ADD-MEMBER button (AC4).
    let add_button = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Add member"),
        (AddMemberButton, DespawnOnExit(RunningState::DebugEditor)),
    );

    // MEMBER-LIST shell — a GTW-412 scroll list. `spawn_scroll_list` puts the MemberListHost
    // marker on the grid ROOT FRAME and RETURNS the ScrollListArea (the clipping/scrolling
    // viewport) — the rows hang in the AREA, the FRAME is parented under the panel (the
    // GTW-421/422 scroll-list parenting rule). The frame is the area's parent once the builder's
    // internal `add_children` flushes; the panel hosts it via a deferred MemberListHost lookup so
    // we never depend on un-flushed parent links inside this builder.
    let list_area = spawn_scroll_list(
        &mut commands,
        scroll_list_colors(&theme),
        (MemberListHost, DespawnOnExit(RunningState::DebugEditor)),
    );

    // Seed one collapsed row per member already in the model (the load-from-registry path),
    // parented into the AREA (not the frame).
    let weapon_keys = sorted_weapon_options(weapons.as_deref());
    let armor_keys = sorted_armor_options(armor.as_deref());
    // Resolve the derivation tuning once (cloned so each row's seed can re-derive); an absent
    // resource falls back to the const-default weights (C3).
    let tuning = tuning.as_deref().cloned().unwrap_or_default();
    if let Some(model) = model.as_ref() {
        for (index, member) in model.members().iter().enumerate() {
            let row = spawn_member_row(
                &mut commands,
                &theme,
                index,
                member,
                &weapon_keys,
                &armor_keys,
                &tuning,
            );
            commands.entity(list_area).add_child(row);
        }
    }

    commands
        .entity(panel)
        .add_children(&[name_field, add_button]);
    commands.entity(root).add_children(&[title, panel]);

    // Parent the scroll-list FRAME (the MemberListHost) under the panel and flex-size it, after
    // the builder's frame→area link has applied — a deferred lookup avoids depending on un-flushed
    // parent links AND on overwriting the builder's grid `Node` (we PATCH it, not replace it). The
    // frame fills the panel's remaining height (flex_grow) yet still shrinks to scroll
    // (`min_height: 0`, the GTW-421 bottom-cramp fix), rather than growing to its content.
    commands.queue(move |world: &mut World| {
        let Some(frame) = world
            .query_filtered::<Entity, With<MemberListHost>>()
            .iter(world)
            .next()
        else {
            return;
        };
        if let Some(mut node) = world.get_mut::<Node>(frame) {
            node.flex_grow = 1.0;
            node.min_height = Val::Px(0.0);
        }
        if let Ok(mut panel_entity) = world.get_entity_mut(panel) {
            panel_entity.add_child(frame);
        }
    });
}

/// Spawns ONE member ROW (GTW-425 collapsed row + GTW-428 expanded stat panel) and returns its
/// root [`Entity`].
///
/// The row is a flex COLUMN: a collapsed HEADER row (the GTW-425 controls) above the GTW-428
/// expanded [`MemberStatPanel`]. The header row carries its [`MemberRow`] marker + [`MemberRowIndex`]
/// and holds (in order): the [`ExpandPip`] toggle, the [`MemberPortrait`] placeholder, the inline
/// [`MemberNameField`] + a [`MemberNameText`] echo, the [`MemberWeaponText`] + the
/// [`MemberWeaponDropdown`], the [`MemberArmorText`] + the [`MemberArmorDropdown`], and the
/// [`DeleteMemberButton`]. BELOW it sits the collapsed-by-default stat panel (C1) holding the eight
/// editable [`AttributeField`]s + the readonly [`DerivedStatText`] displays. Every control carries
/// the same [`MemberRowIndex`] so a commit / selection / press maps back to the member.
/// `weapon_options` / `armor_options` are the pre-sorted dropdown option lists (all loaded keys —
/// C2); `tuning` is the GTW-384 derivation weights the panel seeds its readonly displays from (C3).
pub(in crate::states::running::editor) fn spawn_member_row(
    commands: &mut Commands,
    theme: &GdtfTheme,
    index: usize,
    member: &EditableMember,
    weapon_options: &[DropdownOption<WeaponName>],
    armor_options: &[DropdownOption<ArmorName>],
    tuning: &GangerStatTuning,
) -> Entity {
    let row_index = MemberRowIndex::new(index);
    let text_color = *theme.text.text_color;
    let portrait_color = *theme.panel.color;

    // The member ROOT — a flex COLUMN: the collapsed header row above the expandable stat panel,
    // so the accordion lerp pushes the rows below it DOWN as the panel grows (C1).
    let row = commands
        .spawn((
            MemberRow,
            row_index,
            Themed::new(ThemeRole::Panel),
            member_root_node(),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    // The collapsed HEADER row — the GTW-425 controls laid left→right.
    let header = commands
        .spawn((row_node(), DespawnOnExit(RunningState::DebugEditor)))
        .id();

    // + pip — the expand-toggle CONTROL (renders its toggle state only; GTW-428 owns the panel).
    let pip = commands
        .spawn((
            ExpandPip,
            row_index,
            PipExpanded::new(false),
            Button,
            BackgroundColor(*theme.button.color),
            Text::new(PIP_COLLAPSED_GLYPH.to_owned()),
            TextColor(text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // Portrait placeholder — a plain colored square (no portrait system yet).
    let portrait = commands
        .spawn((
            MemberPortrait,
            BackgroundColor(portrait_color),
            portrait_node(),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // Inline name field (C3), seeded with the member's current name, carrying the row index.
    let name_field = spawn_text_field(
        commands,
        CommittedTextValue::new(member.name().as_str().to_owned()),
        field_colors(theme),
        (
            MemberNameField,
            row_index,
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );
    // Name echo text (C1) — what the dropdown/field commits MUTATE in place (C5). Carries the row
    // index so the commit system finds it.
    let name_text = commands
        .spawn((
            MemberNameText,
            row_index,
            Text::new(member.name().as_str().to_owned()),
            TextColor(text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // Weapon name echo text + weapon dropdown over all loaded keys (C1/C2).
    let (weapon_text, weapon_dropdown) =
        spawn_weapon_loadout(commands, theme, row_index, member.weapon(), weapon_options);

    // Armor name echo text + armor dropdown over all loaded keys (C1/C2).
    let (armor_text, armor_dropdown) =
        spawn_armor_loadout(commands, theme, row_index, member.armor(), armor_options);

    // Delete button (C4) — carries the row index AND the row root so its press removes the right
    // member and despawns exactly this row.
    let delete = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Delete"),
        (
            DeleteMemberButton,
            row_index,
            MemberRowRef::new(row),
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );

    commands.entity(header).add_children(&[
        pip,
        portrait,
        name_field,
        name_text,
        weapon_text,
        weapon_dropdown,
        armor_text,
        armor_dropdown,
        delete,
    ]);

    // The GTW-428 expanded stat panel — the accordion content the pip lerps open / closed (C1),
    // holding the eight editable attribute fields + the readonly derived displays (C2/C3). Spawned
    // collapsed; below the header so its growth pushes later rows DOWN.
    let panel = spawn_member_stat_panel(commands, theme, row_index, member, tuning);

    commands.entity(row).add_children(&[header, panel]);
    row
}

/// Spawns one member row's EXPANDED stat panel (GTW-428 C1/C2/C3) and returns its root
/// [`Entity`] (the GTW-416 [`AccordionContent`](gdtf_ui::AccordionContent) the pip drives).
///
/// The panel IS an accordion content node — it carries the [`AccordionContent`] marker that
/// `drive_accordions` phase 2 iterates (WITHOUT it the height lerp never runs), plus
/// [`MemberStatPanel`] + the row's [`MemberRowIndex`] + the shared `drive_accordions` animation
/// components ([`AccordionAnim::Collapsed`], [`AccordionProgress`]`(0.0)`). The open lerp animates
/// up to the PER-INSTANCE [`AccordionExpandedVh`]`(`[`STAT_PANEL_EXPANDED_VH`]`)` waypoint, then —
/// because the panel also carries [`AccordionContentFit`] — settles to a [`Val::Auto`](bevy::ui::Val)
/// CONTENT-FIT height so its sixteen stat lines all show regardless of the rendered font / padding
/// (the GTW-428 round-2 layout fix — a fixed `Vh` ceiling clipped the taller column's bottom lines
/// at the live window size). It starts at the collapsed height, CLIPPING its overflow so it shows
/// nothing until the pip toggles it open (the height lerp reveals it; at rest `Auto` exactly fits
/// the content so the clip cuts nothing).
///
/// The sixteen lines lay in TWO COLUMNS (the GTW-428 layout fix): a LEFT column of the eight
/// editable [`AttributeField`] numeric fields (each clamped to [`attribute_range`], seeded from the
/// member's current attribute — C2) and a RIGHT column of the eight readonly [`DerivedStatText`]
/// displays seeded from the GTW-384 [`derive_stats`] pipeline over the member's current attributes
/// (C3). Two columns of eight halve the panel's vertical extent versus one column of sixteen, so
/// the open panel clears the now-fitting height with every line visible and legible.
fn spawn_member_stat_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    member: &EditableMember,
    tuning: &GangerStatTuning,
) -> Entity {
    // The panel content node — collapsed + clipping, carrying the accordion animation state the
    // shared `drive_accordions` lerps and the pip toggle flips (C1), plus the per-instance expanded
    // height so it opens tall enough to show its sixteen lines (the GTW-428 layout fix).
    let panel = commands
        .spawn((
            MemberStatPanel,
            AccordionContent,
            // Settle the rest-open panel to a CONTENT-FIT (`Val::Auto`) height so all sixteen
            // lines show regardless of the rendered font / padding — a fixed `Vh` ceiling could
            // not reliably clear the taller (eight-attribute-field) column at the live window
            // size (the GTW-428 round-2 QA defect). The lerp still opens through `Vh` toward the
            // per-instance target below for a visible animation; `Auto` takes over only at rest.
            AccordionContentFit,
            row_index,
            AccordionAnim::Collapsed,
            AccordionProgress::new(0.0),
            AccordionExpandedVh::new(STAT_PANEL_EXPANDED_VH),
            BackgroundColor(*theme.panel.color),
            stat_panel_node(),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // The two side-by-side stat columns: editable attributes (left) | readonly derived (right).
    let attributes_column = commands
        .spawn((stat_column_node(), DespawnOnExit(RunningState::DebugEditor)))
        .id();
    let derived_column = commands
        .spawn((stat_column_node(), DespawnOnExit(RunningState::DebugEditor)))
        .id();

    // LEFT column — the eight editable attribute fields (C2).
    for attribute in BaseAttribute::ALL {
        let field = spawn_attribute_field(commands, theme, row_index, member, attribute);
        commands.entity(attributes_column).add_child(field);
    }

    // RIGHT column — the eight readonly derived-stat displays, seeded from the real GTW-384
    // pipeline (C3).
    let stats: DerivedStats = derive_stats(&member.attributes(), tuning);
    for stat in DerivedStat::ALL {
        let display = spawn_derived_display(commands, theme, row_index, stat, &stats);
        commands.entity(derived_column).add_child(display);
    }

    commands
        .entity(panel)
        .add_children(&[attributes_column, derived_column]);
    panel
}

/// Spawn one EDITABLE base-attribute row inside a stat panel: a label beside a clamped numeric
/// field seeded with the member's current value (GTW-428 C2). Returns the row [`Entity`]. The
/// numeric field carries the [`AttributeField`] marker + the row's [`MemberRowIndex`] + the
/// [`BaseAttribute`] so a [`NumericFieldCommitted`](gdtf_ui::NumericFieldCommitted)`<f32>` maps to
/// the right member's right attribute.
fn spawn_attribute_field(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    member: &EditableMember,
    attribute: BaseAttribute,
) -> Entity {
    let row = commands
        .spawn((stat_line_node(), DespawnOnExit(RunningState::DebugEditor)))
        .id();
    let label = commands
        .spawn((
            Text::new(attribute.label().to_owned()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    let field = spawn_numeric_field(
        commands,
        member.attribute(attribute),
        attribute_range(),
        field_colors(theme),
        (
            AttributeField,
            row_index,
            attribute,
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );
    commands.entity(row).add_children(&[label, field]);
    row
}

/// Spawn one READONLY derived-stat row inside a stat panel: a label beside a [`DerivedStatText`]
/// value node seeded from the GTW-384 pipeline output for the member's current attributes (GTW-428
/// C3). Returns the row [`Entity`]. The value node carries the [`DerivedStatText`] marker + the
/// row's [`MemberRowIndex`] + the [`DerivedStat`] so the recompute mutates the right node in place
/// (NOT a numeric field — derived stats are readonly, C2).
fn spawn_derived_display(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    stat: DerivedStat,
    stats: &DerivedStats,
) -> Entity {
    let row = commands
        .spawn((stat_line_node(), DespawnOnExit(RunningState::DebugEditor)))
        .id();
    let label = commands
        .spawn((
            Text::new(stat.label().to_owned()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    let value = commands
        .spawn((
            DerivedStatText,
            row_index,
            stat,
            Text::new(format_derived(stats, stat)),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    commands.entity(row).add_children(&[label, value]);
    row
}

/// Spawn one row's WEAPON loadout column: the [`MemberWeaponText`] echo (the member's current
/// weapon key) beside the [`MemberWeaponDropdown`] over ALL loaded keys (C1/C2). Both carry the
/// row's [`MemberRowIndex`]; the dropdown opens on the member's current key (or the first option if
/// it is not a loaded key). Returns `(text, dropdown)`.
fn spawn_weapon_loadout(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    current: &WeaponName,
    options: &[DropdownOption<WeaponName>],
) -> (Entity, Entity) {
    let text = commands
        .spawn((
            MemberWeaponText,
            row_index,
            Text::new((**current).clone()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    let dropdown = spawn_dropdown(
        commands,
        options.to_vec(),
        option_index_of(options, current),
        dropdown_colors(theme),
        (
            MemberWeaponDropdown,
            row_index,
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );
    (text, dropdown)
}

/// Spawn one row's ARMOR loadout column: the [`MemberArmorText`] echo beside the
/// [`MemberArmorDropdown`] over ALL loaded armor keys (C1/C2). The armor mirror of
/// [`spawn_weapon_loadout`]. Returns `(text, dropdown)`.
fn spawn_armor_loadout(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    current: &ArmorName,
    options: &[DropdownOption<ArmorName>],
) -> (Entity, Entity) {
    let text = commands
        .spawn((
            MemberArmorText,
            row_index,
            Text::new((**current).clone()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();
    let dropdown = spawn_dropdown(
        commands,
        options.to_vec(),
        option_index_of(options, current),
        dropdown_colors(theme),
        (
            MemberArmorDropdown,
            row_index,
            DespawnOnExit(RunningState::DebugEditor),
        ),
    );
    (text, dropdown)
}

/// Build the pre-sorted weapon dropdown option list — ALL loaded
/// [`WeaponName`](gdtf_battle_sim::WeaponName) keys (C2), sorted by name for a stable order (the
/// `HashMap` `keys()` order is unspecified). Each option's identity IS its key; its label is the
/// key string. Returns an empty list when the registry is absent (degraded, never panics).
pub(in crate::states::running::editor) fn sorted_weapon_options(
    weapons: Option<&WeaponRegistry>,
) -> Vec<DropdownOption<WeaponName>> {
    let Some(weapons) = weapons else {
        return Vec::new();
    };
    let mut keys: Vec<&WeaponName> = weapons.keys().collect();
    keys.sort_by_key(|key| (***key).clone());
    keys.into_iter()
        .map(|key| DropdownOption::new(key.clone(), (**key).clone()))
        .collect()
}

/// Build the pre-sorted armor dropdown option list — ALL loaded
/// [`ArmorName`](gdtf_battle_sim::ArmorName) keys (C2), sorted by name. The armor mirror of
/// [`sorted_weapon_options`].
pub(in crate::states::running::editor) fn sorted_armor_options(
    armor: Option<&ArmorRegistry>,
) -> Vec<DropdownOption<ArmorName>> {
    let Some(armor) = armor else {
        return Vec::new();
    };
    let mut keys: Vec<&ArmorName> = armor.keys().collect();
    keys.sort_by_key(|key| (***key).clone());
    keys.into_iter()
        .map(|key| DropdownOption::new(key.clone(), (**key).clone()))
        .collect()
}

/// The index of `id` in `options` (matched by identity), or `0` if absent — the dropdown's
/// initially-shown slot. A member whose current key is not a loaded option simply shows the first.
fn option_index_of<T: gdtf_ui::OptionId>(options: &[DropdownOption<T>], id: &T) -> usize {
    options
        .iter()
        .position(|opt| opt.id() == id)
        .unwrap_or_default()
}

/// The [`FieldColors`] a text field paints with, read from the theme (background / text from the
/// panel + body-text sub-themes, the caret from the body text). Pure UI plumbing colors.
fn field_colors(theme: &GdtfTheme) -> FieldColors {
    FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    }
}

/// The [`DropdownColors`] a member-row dropdown paints with, from the theme. Pure UI plumbing.
fn dropdown_colors(theme: &GdtfTheme) -> DropdownColors {
    DropdownColors {
        control_bg: *theme.button.color,
        text:       *theme.text.text_color,
        popup_bg:   *theme.panel.color,
        option_bg:  *theme.button.color,
    }
}

/// The [`ScrollListColors`] the member-list scroll list paints with, from the theme. Pure UI
/// plumbing.
fn scroll_list_colors(theme: &GdtfTheme) -> ScrollListColors {
    ScrollListColors {
        area:  *theme.panel.color,
        track: *theme.button.color,
        thumb: *theme.text.text_color,
    }
}

/// One member's ROOT [`Node`]: a full-width flex COLUMN stacking the collapsed header row above
/// the expandable stat panel, so the accordion lerp grows the panel downward and pushes the rows
/// below it DOWN (C1). Relative units.
fn member_root_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// One collapsed HEADER row's [`Node`]: a full-width flex ROW laying its controls left→right,
/// centred on the cross axis, with a small gap + padding so the controls read as one row. Relative
/// units.
fn row_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Vw(0.6),
        padding: bevy::ui::UiRect::all(Val::Vh(0.6)),
        ..default()
    }
}

/// The expanded stat panel's [`Node`] (the GTW-416 accordion content): a full-width flex ROW
/// holding the two stat COLUMNS side by side (the GTW-428 2-column layout fix), starting at zero
/// height and CLIPPING its overflow, so a collapsed panel shows nothing and the `drive_accordions`
/// lerp reveals it by animating the height up to the panel's per-instance
/// [`AccordionExpandedVh`](gdtf_ui::AccordionExpandedVh) target (C1). The shared driver writes the
/// height while animating; the panel is seeded collapsed (`Val::Vh(0.0)`). A column gap separates
/// the two columns. Relative units.
fn stat_panel_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Vh(0.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Start,
        column_gap: Val::Vw(1.2),
        padding: bevy::ui::UiRect::all(Val::Vh(0.6)),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// One stat COLUMN's [`Node`] inside the expanded panel: a flex COLUMN that takes an equal share of
/// the panel width (`flex_basis: 0` + `flex_grow: 1`) and stacks its eight stat lines top-to-bottom
/// with the calibrated inter-line gap (the GTW-428 2-column layout fix). A `min_height: 0` lets the
/// column shrink below its content while the panel is mid-lerp (the flex-item bottom-cramp guard).
/// Relative units.
fn stat_column_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        flex_basis: Val::Px(0.0),
        flex_grow: 1.0,
        min_height: Val::Px(0.0),
        row_gap: Val::Vh(STAT_PANEL_GAP_VH),
        ..default()
    }
}

/// One stat LINE's [`Node`] inside the panel (a label beside its field / value): a full-width flex
/// ROW with a gap so the label and the value read as one line. Relative units.
fn stat_line_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Vw(0.6),
        ..default()
    }
}

/// The portrait placeholder's [`Node`]: a small square sized in viewport-relative units (the
/// palette tile-sprite precedent), so it scales with the window.
fn portrait_node() -> Node {
    Node {
        width: Val::Vw(PORTRAIT_SIDE_VW),
        height: Val::Vw(PORTRAIT_SIDE_VW),
        ..default()
    }
}
