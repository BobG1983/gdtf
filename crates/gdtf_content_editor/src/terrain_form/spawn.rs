//! The TERRAIN-mode form **layout** (GTW-474): spawn the form's widgets into the three regions'
//! TERRAIN-mode content containers `OnEnter(Editing)`.
//!
//! Reuses the landed `gdtf_ui` widgets: [`spawn_segmented_control`] (kind + height band),
//! [`spawn_text_field`] (display name), [`spawn_numeric_field`] (HP + armor stats),
//! [`spawn_dropdown`] (footfall), and plain [`Button`] rows (the graphic-role picker + the tag
//! multi-select + the save button). The widgets parent into the TERRAIN container under each
//! region so they show only in TERRAIN mode (the mode toggle flips the container's
//! [`Visibility`]).
//!
//! LAYOUT (logged sub-decisions, defensible readings of the mockup):
//! - LEFT region (TERRAIN container) = the GRAPHIC-ROLE PICKER — a column of `Button` rows, one
//!   per `TileRoles` role key, the selected carrying `ActiveButton` (the palette-row precedent).
//!   The mockup's "scroll-list or grid of graphic keys"; a button column inside the already-
//!   scrollable left region is the lowest-churn faithful reading.
//! - CENTER region (TERRAIN container) = the KIND segmented control + the HEIGHT-BAND segmented
//!   control.
//! - RIGHT region (TERRAIN container) = the field stack: display name, HP, armor protection,
//!   armor hardness, the footfall dropdown (gated to Slab — C2), the tag multi-select, and the
//!   read-only UUID + the "Save terrain" button.
//! - STAT region (TERRAIN container) = the read-only `.terrain_def.ron` live PREVIEW.

use bevy::{prelude::*, ui::FlexDirection};
use gdtf_battle_sim::terrain::def::TerrainTag;
use gdtf_ui::{
    CommittedTextValue, DropdownColors, DropdownOption, FieldColors, NumericRange, Orientation,
    SegmentColors, SegmentLabel, spawn_dropdown, spawn_numeric_field, spawn_segmented_control,
    spawn_text_field, theme::GdtfTheme,
};

use super::types::{
    ArmorInput, FootfallChoice, HpInput, SaveTerrainButton, TerrainArmorHardField,
    TerrainArmorProtField, TerrainBandTabs, TerrainDraft, TerrainFootfallPicker,
    TerrainGraphicChoice, TerrainGraphicPicker, TerrainHpField, TerrainKindChoice, TerrainKindTabs,
    TerrainNameField, TerrainRonPreview, TerrainTagToggle, TerrainUuidText,
};
use crate::{
    CanvasRegion, LeftPaletteRegion, RightPanelRegion, StatRegion, mode::TerrainModeContent,
    mode_host::mode_host_under_region,
};

/// The lower / upper HP clamp the HP numeric field accepts.
const HP_MIN: u32 = 0;
/// The upper HP clamp — a generous ceiling so any authored structural pool fits.
const HP_MAX: u32 = 1000;
/// The lower / upper armor clamp the armor numeric fields accept.
const ARMOR_MIN: i32 = 0;
/// The upper armor clamp.
const ARMOR_MAX: i32 = 100;

/// The closed tag set the multi-select offers, in display order (C2).
const TAG_ORDER: [TerrainTag; 4] = [
    TerrainTag::Openable,
    TerrainTag::BlocksVision,
    TerrainTag::BlocksPathfinding,
    TerrainTag::Indestructible,
];

/// The display label for a tag toggle.
const fn tag_label(tag: TerrainTag) -> &'static str {
    match tag {
        TerrainTag::Openable => "Openable",
        TerrainTag::BlocksVision => "Blocks Vision",
        TerrainTag::BlocksPathfinding => "Blocks Pathfinding",
        TerrainTag::Indestructible => "Indestructible",
    }
}

/// `OnEnter(Editing)`: spawn the TERRAIN form's widgets into the three regions' TERRAIN-mode
/// content containers (GTW-474 C2).
///
/// Gated on the live [`GdtfTheme`] (for the control colors). Reads the [`TerrainDraft`]'s default
/// values to seed the widgets (the draft is inserted in the same `OnEnter` buffer, so this seeds
/// from [`TerrainDraft::default`] to avoid the command-flush race — the GTW-421 precedent). Each
/// widget carries its identity marker so the form's drive systems map a commit to the draft.
pub(crate) fn spawn_terrain_form(mut commands: Commands, theme: Res<GdtfTheme>) {
    let draft = TerrainDraft::default();
    let field_colors = FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    };
    let dropdown_colors = DropdownColors {
        control_bg:          *theme.panel.color,
        text:                *theme.text.text_color,
        popup_bg:            *theme.panel.color,
        option_bg:           *theme.panel.border_color,
        option_highlight_bg: *theme.button.hover,
    };
    let segment_colors = SegmentColors {
        active_bg:   *theme.button.hover,
        active_text: *theme.text.text_color,
        base_bg:     *theme.panel.color,
        base_text:   *theme.text.text_color,
    };
    let label_color = *theme.text.text_color;

    let button_bg = *theme.panel.border_color;

    spawn_left_picker(&mut commands, &theme, &draft);
    spawn_center_controls(&mut commands, segment_colors, &draft);
    spawn_right_fields(
        &mut commands,
        field_colors,
        dropdown_colors,
        label_color,
        button_bg,
        &draft,
    );
    spawn_stat_preview(&mut commands, label_color);
}

/// LEFT region: the graphic-role picker — a `Button` row per role key, the draft's role
/// pre-selected (carrying [`ActiveButton`](gdtf_ui::ActiveButton)).
fn spawn_left_picker(commands: &mut Commands, theme: &GdtfTheme, draft: &TerrainDraft) {
    let row_bg = *theme.panel.color;
    let text_color = *theme.text.text_color;
    let rows: Vec<Entity> = TerrainGraphicChoice::ALL
        .iter()
        .map(|choice| {
            let mut row = commands.spawn((
                Button,
                TerrainGraphicPicker,
                *choice,
                BackgroundColor(row_bg),
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Vh(0.6)),
                    ..default()
                },
            ));
            row.with_child((Text::new(choice.key()), TextColor(text_color)));
            if *choice == draft.graphic() {
                row.insert(gdtf_ui::ActiveButton);
            }
            row.id()
        })
        .collect();
    let header = commands
        .spawn((
            Text::new("Graphic role"),
            TextColor(text_color),
            header_node(),
        ))
        .id();
    let column = commands
        .spawn(form_column_node())
        .add_child(header)
        .add_children(&rows)
        .id();
    parent_under_terrain::<LeftPaletteRegion>(commands, column);
}

/// CENTER region: the KIND segmented control + the HEIGHT-BAND segmented control.
fn spawn_center_controls(commands: &mut Commands, colors: SegmentColors, draft: &TerrainDraft) {
    let kind_labels: Vec<SegmentLabel> = TerrainKindChoice::SEGMENT_ORDER
        .iter()
        .map(|kind| SegmentLabel::new(kind.label()))
        .collect();
    let kind = spawn_segmented_control(
        commands,
        &kind_labels,
        draft.kind().segment_index(),
        colors,
        Orientation::Horizontal,
        TerrainKindTabs,
    );
    let band_labels = [
        SegmentLabel::new("Low"),
        SegmentLabel::new("Mid"),
        SegmentLabel::new("High"),
    ];
    let band = spawn_segmented_control(
        commands,
        &band_labels,
        band_segment_index(draft.height_band()),
        colors,
        Orientation::Horizontal,
        TerrainBandTabs,
    );
    let column = commands
        .spawn(form_column_node())
        .add_children(&[kind, band])
        .id();
    parent_under_terrain::<CanvasRegion>(commands, column);
}

/// RIGHT region: the field stack (name, HP, armor, footfall, tags, UUID, save).
fn spawn_right_fields(
    commands: &mut Commands,
    field_colors: FieldColors,
    dropdown_colors: DropdownColors,
    label_color: Color,
    button_bg: Color,
    draft: &TerrainDraft,
) {
    let name = spawn_text_field(
        commands,
        CommittedTextValue::new(draft.display_name()),
        field_colors,
        TerrainNameField,
    );
    let name_group = labeled(commands, "Name", label_color, name);

    let hp = spawn_numeric_field(
        commands,
        HpInput::new(*draft.cover_hp()),
        NumericRange::new(HpInput::new(HP_MIN), HpInput::new(HP_MAX)),
        field_colors,
        TerrainHpField,
    );
    let hp_group = labeled(commands, "Max HP", label_color, hp);

    let prot = spawn_numeric_field(
        commands,
        ArmorInput::new(*draft.armor_protection()),
        NumericRange::new(ArmorInput::new(ARMOR_MIN), ArmorInput::new(ARMOR_MAX)),
        field_colors,
        TerrainArmorProtField,
    );
    let prot_group = labeled(commands, "Armor protection", label_color, prot);

    let hard = spawn_numeric_field(
        commands,
        ArmorInput::new(*draft.armor_hardness()),
        NumericRange::new(ArmorInput::new(ARMOR_MIN), ArmorInput::new(ARMOR_MAX)),
        field_colors,
        TerrainArmorHardField,
    );
    let hard_group = labeled(commands, "Armor hardness", label_color, hard);

    let footfall_group = spawn_footfall_group(commands, dropdown_colors, label_color, draft);
    let tags_group = spawn_tags_group(commands, field_colors, label_color, draft);
    let uuid_text = commands
        .spawn((
            TerrainUuidText,
            Text::new("UUID: (generated on save)"),
            TextColor(label_color),
            header_node(),
        ))
        .id();
    let save_button = spawn_save_button(commands, button_bg, label_color);

    let column = commands
        .spawn(form_column_node())
        .add_children(&[
            name_group,
            hp_group,
            prot_group,
            hard_group,
            footfall_group,
            tags_group,
            uuid_text,
            save_button,
        ])
        .id();
    parent_under_terrain::<RightPanelRegion>(commands, column);
}

/// The footfall dropdown group — spawned ALWAYS (C2). The footfall gate adds/removes
/// `DisabledButton` on it driven by the draft kind, so it is greyed + non-pressable for any
/// non-slab kind. Its options include an explicit `None`.
fn spawn_footfall_group(
    commands: &mut Commands,
    dropdown_colors: DropdownColors,
    label_color: Color,
    draft: &TerrainDraft,
) -> Entity {
    let footfall_options: Vec<DropdownOption<FootfallChoice>> = FootfallChoice::ALL
        .iter()
        .map(|choice| DropdownOption::new(*choice, choice.label()))
        .collect();
    let selected = FootfallChoice::ALL
        .iter()
        .position(|choice| *choice == draft.footfall())
        .unwrap_or(0);
    let footfall = spawn_dropdown(
        commands,
        footfall_options,
        selected,
        dropdown_colors,
        TerrainFootfallPicker,
    );
    labeled(commands, "Footfall (Slab only)", label_color, footfall)
}

/// The tag multi-select group — one `Button` per tag, `ActiveButton` when selected (toggled by
/// the form's tag-toggle system).
fn spawn_tags_group(
    commands: &mut Commands,
    field_colors: FieldColors,
    label_color: Color,
    draft: &TerrainDraft,
) -> Entity {
    let tag_rows: Vec<Entity> = TAG_ORDER
        .iter()
        .map(|tag| {
            let mut row = commands.spawn((
                Button,
                TerrainTagToggle::new(*tag),
                BackgroundColor(field_colors.background),
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Vh(0.5)),
                    margin: UiRect::bottom(Val::Vh(0.3)),
                    ..default()
                },
            ));
            row.with_child((Text::new(tag_label(*tag)), TextColor(label_color)));
            if draft.has_tag(*tag) {
                row.insert(gdtf_ui::ActiveButton);
            }
            row.id()
        })
        .collect();
    let header = commands
        .spawn((Text::new("Tags"), TextColor(label_color), header_node()))
        .id();
    commands
        .spawn(group_node())
        .add_child(header)
        .add_children(&tag_rows)
        .id()
}

/// The "Save terrain" button.
fn spawn_save_button(commands: &mut Commands, button_bg: Color, label_color: Color) -> Entity {
    commands
        .spawn((
            SaveTerrainButton,
            Button,
            BackgroundColor(button_bg),
            Node {
                width: Val::Percent(100.0),
                margin: UiRect::top(Val::Vh(0.6)),
                padding: UiRect::all(Val::Vh(0.6)),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child((Text::new("Save terrain"), TextColor(label_color)))
        .id()
}

/// STAT region: the read-only `.terrain_def.ron` live preview text node.
fn spawn_stat_preview(commands: &mut Commands, label_color: Color) {
    let preview = commands
        .spawn((
            TerrainRonPreview,
            Text::new(".terrain_def.ron preview"),
            TextColor(label_color),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Vh(1.0)),
                ..default()
            },
        ))
        .id();
    parent_under_terrain::<StatRegion>(commands, preview);
}

/// Wrap `control` in a labeled COLUMN group (a small text label above the control), stretched to
/// the panel width — the `right_panel` `spawn_field_group` shape.
fn labeled(commands: &mut Commands, label: &str, color: Color, control: Entity) -> Entity {
    let label_node = commands
        .spawn((Text::new(label), TextColor(color), header_node()))
        .id();
    commands
        .spawn(group_node())
        .add_children(&[label_node, control])
        .id()
}

/// Parent `content` under the TERRAIN-mode content container of the region marked `Region` (a
/// deferred command, the shell-parenting idiom).
fn parent_under_terrain<Region: Component>(commands: &mut Commands, content: Entity) {
    commands.queue(move |world: &mut World| {
        let Some(host) = mode_host_under_region::<Region, TerrainModeContent>(world) else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            host_entity.add_child(content);
        }
    });
}

/// The form content COLUMN [`Node`]: full-width, top-aligned, padded, with row gaps so groups
/// breathe. Relative units (no fixed `Px`).
fn form_column_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Start,
        padding: UiRect::all(Val::Vh(1.0)),
        row_gap: Val::Vh(1.0),
        ..default()
    }
}

/// One labeled GROUP's [`Node`]: a flex COLUMN (label above control) filling the width.
fn group_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: Val::Vh(0.4),
        ..default()
    }
}

/// A small header/label [`Node`]: a little bottom margin so the label sits above its control.
fn header_node() -> Node {
    Node {
        margin: UiRect::bottom(Val::Vh(0.3)),
        ..default()
    }
}

/// The [`HeightBand`](gdtf_battle_sim::cover::HeightBand) → band-segment-index map (Low/Mid/High).
const fn band_segment_index(band: gdtf_battle_sim::cover::HeightBand) -> usize {
    match band {
        gdtf_battle_sim::cover::HeightBand::Low => 0,
        gdtf_battle_sim::cover::HeightBand::Mid => 1,
        gdtf_battle_sim::cover::HeightBand::High => 2,
    }
}
