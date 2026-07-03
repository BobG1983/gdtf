//! GTW-428: headless behavioral tests for the in-app gang-editor's EXPANDED per-member stat table.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin` —
//! which registers `drive_accordions` — and the real `EditorScenePlugin` wired through
//! `ScenesPlugin`), seeded with a theme + a [`GangerStatTuning`] so the editor screen spawns with
//! the GTW-384 derivation available. They assert on the WORLD and the model resource (the
//! system-effects) — never on rendering or real device input.
//!
//! Editing is driven through the REAL input MESSAGES the widgets raise
//! ([`NumericFieldCommitted`]`<f32>`) or the real action system via a set [`Interaction::Pressed`]
//! plus an `update()` (the accepted headless idiom — `MinimalPlugins` has no `ui_focus_system` to
//! clobber it). Each assertion is pin-discriminating: it would FAIL if its clause were reverted.
//!
//! Coverage (the ticket's three headless tests):
//!
//! - [`pip_press_drives_the_accordion_target`] — toggling the pip flips its `PipExpanded` AND
//!   drives the matching stat panel's `AccordionAnim` toward `Expanding` (C1).
//! - [`expanded_panel_has_eight_clamped_attribute_fields`] — the panel holds the eight editable
//!   attribute fields, each carrying a `NumericRange<f32>` that clamps out-of-range input (C2).
//! - [`editing_attribute_recomputes_derived_to_pipeline_output`] — a real `NumericFieldCommitted`
//!   on an attribute updates the member attribute AND every displayed derived stat equals the
//!   GTW-384 `derive_stats` output for the new attributes (C3, pin-discriminating: a missing
//!   recompute or stale attributes would leave the OLD derived text and fail).

use bevy::{
    app::App,
    ecs::{component::Component, entity::Entity, hierarchy::Children},
    prelude::{Text, With},
    state::state::NextState,
    ui::{Interaction, Node, Val},
};
use gdtf_app::test_support::{
    AddMemberButton, AppState, AttributeField, BaseAttribute, DerivedStat, DerivedStatText,
    EditableGang, ExpandPip, MemberRowIndex, MemberStatPanel, PipExpanded, RunningState,
};
use gdtf_battle_sim::{
    Accuracy, Aim, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece,
    ArmorProtection, ArmorRegistry, ArmorSpec, ArmorType, BaseSpread, Cool, DamageType,
    DerivedStats, FatalBias, FireMode, FireModeSpec, GangerAttributes, GangerStatTuning, Grit,
    Kickback, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Reflexes,
    ReloadTu, Shove, Speed, Stable, Strength, Toughness, TrajectoryStyle, WeaponDamage, WeaponName,
    WeaponPunch, WeaponShred, WeaponSpec, derive_stats, ganger::Luck, weapon::Handedness,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::{
    AccordionAnim, AccordionProgress, CommittedNumericValue, NumericFieldCommitted, NumericRange,
    theme::default_theme,
};

/// A weapon key (the dropdowns need at least one option to spawn cleanly).
const WEAPON_A: &str = "Autogun";
/// An armor key (mirrors [`WEAPON_A`]).
const ARMOR_A: &str = "Flak";

/// An arbitrary weapon spec (NOT shipped tuning) — the editor only reads the KEY.
fn arbitrary_weapon() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.25),
        accuracy:    Accuracy::new(1.0),
        kickback:    Kickback::new(0.4),
        fatal_bias:  FatalBias::new(0.0),
        damage:      WeaponDamage::new(12),
        punch:       WeaponPunch::new(5),
        shred:       WeaponShred::new(3),
        damage_type: DamageType::Kinetic,
        magazine:    Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:   FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:      Stable::new(false),
        shove:       Shove::new(false),
        handedness:  Handedness::OneHanded,
        trajectory:  TrajectoryStyle::Straight,
        // GTW-554: no slots declared / no attachments fitted (the empty defaults).
        slots:       gdtf_battle_sim::WeaponSlots::default(),
        attachments: Vec::new(),
        dot:         None,
        on_death:    None,
    }
}

/// A [`WeaponRegistry`](gdtf_battle_sim::WeaponRegistry) of one test weapon key.
fn weapon_registry() -> gdtf_battle_sim::WeaponRegistry {
    gdtf_battle_sim::WeaponRegistry::new([(
        WeaponName::new(WEAPON_A.to_owned()),
        arbitrary_weapon(),
    )])
}

/// An arbitrary armor spec (NOT shipped tuning).
const fn arbitrary_armor() -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(1),
        ArmorIntegrity::new(2),
        ArmorHardness::new(3),
        ArmorType::DEFAULT,
    ))
}

/// An [`ArmorRegistry`] of one test armor key.
fn armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(ArmorName::new(ARMOR_A.to_owned()), arbitrary_armor())])
}

/// Builds a headless app driven into [`RunningState::DebugEditor`] with the editor screen spawned,
/// the weapon / armor registries seeded (so the per-member dropdowns have options), and the
/// [`GangerStatTuning`] seeded (so the production recompute and the test compute through the SAME
/// derivation weights — C3). Starts in `AppState::Running` (default sub-state `Menu`), seeds the
/// resources before the first update so the `OnEnter` spawn sees them, then sets the `DebugEditor`
/// transition and pumps a few updates so the screen + any deferred parenting flush.
fn editor_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(weapon_registry());
    app.world_mut().insert_resource(armor_registry());
    app.world_mut().insert_resource(test_tuning());
    app.update();
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugEditor);
    app.update();
    app.update();
    app
}

/// The derivation tuning the test and the production code share — the const default
/// [`GangerStatTuning`] (the stats.md flat weights). Both the editor's recompute and the test's
/// expected derivation read THIS, so the comparison is exact.
fn test_tuning() -> GangerStatTuning {
    GangerStatTuning::default()
}

/// Presses "Add member" through the real add-member system (set `Interaction::Pressed` + a couple
/// `update()`s, the headless idiom — the deferred parent-into-area command needs a second flush).
fn press_add_member(app: &mut App) {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<AddMemberButton>>();
    let button = q.iter(app.world()).next().unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
    app.update();
    app.update();
}

/// The first entity carrying BOTH marker `M` and a [`MemberRowIndex`] equal to `index`.
fn control_for_row<M: Component>(app: &mut App, index: usize) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &MemberRowIndex), With<M>>();
    q.iter(app.world())
        .find(|(_, row_index)| ***row_index == index)
        .map(|(entity, _)| entity)
}

/// Every `AttributeField` entity carrying `MemberRowIndex == index`, paired with its
/// [`BaseAttribute`].
fn attribute_fields_for_row(app: &mut App, index: usize) -> Vec<(Entity, BaseAttribute)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &MemberRowIndex, &BaseAttribute), With<AttributeField>>();
    q.iter(app.world())
        .filter(|(_, row_index, _)| ***row_index == index)
        .map(|(entity, _, attribute)| (entity, *attribute))
        .collect()
}

/// The [`Text`] string of the readonly [`DerivedStatText`] node for `(index, stat)`.
fn derived_text(app: &mut App, index: usize, stat: DerivedStat) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&MemberRowIndex, &DerivedStat, &Text), With<DerivedStatText>>();
    q.iter(app.world())
        .find(|(row_index, kind, _)| ***row_index == index && **kind == stat)
        .map(|(_, _, text)| text.0.clone())
}

/// Format one derived stat the SAME way the production `derived_display` formatter does (the f32
/// skill stats to one decimal, the integer pools as bare integers) — kept in lockstep so the test
/// asserts against the production rendering of the SHARED `derive_stats` output (C3).
fn expected_derived(stats: &DerivedStats, stat: DerivedStat) -> String {
    match stat {
        DerivedStat::Shooting => format!("{:.1}", *stats.shooting),
        DerivedStat::Fight => format!("{:.1}", *stats.fight),
        DerivedStat::Reactions => format!("{:.1}", *stats.reactions),
        DerivedStat::Morale => format!("{:.1}", *stats.morale),
        DerivedStat::Tu => format!("{}", *stats.tu),
        DerivedStat::Hp => format!("{}", *stats.hp),
        DerivedStat::Wounds => format!("{}", *stats.wounds),
        DerivedStat::Bottle => format!("{}", *stats.bottle),
    }
}

/// The viewport-height (`Vh`) magnitude of a [`Node`]'s `height`, or `None` if it is not a
/// `Val::Vh` — the stat panel's height is a relative `Vh` while the accordion lerp is in flight
/// (C1). Once a content-fit panel settles fully open its height becomes [`Val::Auto`], so a
/// settled member panel returns `None` here (see [`panel_height_is_auto`]).
fn panel_height_vh(app: &App, panel: Entity) -> Option<f32> {
    match app.world().get::<Node>(panel).map(|node| node.height) {
        Some(Val::Vh(vh)) => Some(vh),
        _ => None,
    }
}

/// Whether the panel's `Node.height` is [`Val::Auto`] — the CONTENT-FIT rest-open height the
/// shared `drive_accordions` switches a settled [`AccordionContentFit`] section to, so the
/// rest-open panel sizes to its exact content (the GTW-428 round-2 layout fix).
fn panel_height_is_auto(app: &App, panel: Entity) -> bool {
    matches!(
        app.world().get::<Node>(panel).map(|node| node.height),
        Some(Val::Auto)
    )
}

/// C1: toggling the `+` pip flips `PipExpanded`, drives the matching member stat panel's
/// `AccordionAnim` toward expanding, AND the shared `drive_accordions` lerp then actually opens the
/// panel — its `AccordionProgress` and its `Node.height` both move strictly OFF zero over time
/// (the height growth is what pushes the rows below it down).
///
/// Pin: a no-op pip (no `PipExpanded` flip) fails the flag assert; a pip that flips its own state
/// but does NOT drive the panel leaves the panel `Collapsed` and fails the `AccordionAnim` assert;
/// a panel that is NOT an `AccordionContent` (so `drive_accordions` phase 2 never iterates it)
/// leaves `AccordionProgress` at `0.0` and `Node.height` at `Vh(0.0)` and fails the lerp asserts —
/// this is the half of C1 the pre-fix code regressed.
#[test]
fn pip_press_drives_the_accordion_target() {
    let mut app = editor_app();
    press_add_member(&mut app);

    let pip = control_for_row::<ExpandPip>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    let panel = control_for_row::<MemberStatPanel>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    // Precondition: a fresh row's pip is collapsed and its panel accordion is at rest collapsed,
    // at zero progress and zero height.
    assert_eq!(
        app.world().get::<PipExpanded>(pip).map(|p| **p),
        Some(false),
        "a fresh pip starts collapsed",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(panel),
        Some(&AccordionAnim::Collapsed),
        "a fresh stat panel starts at rest collapsed",
    );
    assert_eq!(
        app.world().get::<AccordionProgress>(panel).map(|p| **p),
        Some(0.0),
        "a fresh stat panel starts at zero accordion progress",
    );
    assert_eq!(
        panel_height_vh(&app, panel),
        Some(0.0),
        "a fresh stat panel starts at zero height (Vh(0.0))",
    );

    // Press the pip (the headless idiom — set Pressed then update).
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(pip) {
        *interaction = Interaction::Pressed;
    }
    app.update();

    assert_eq!(
        app.world().get::<PipExpanded>(pip).map(|p| **p),
        Some(true),
        "pressing the pip flips PipExpanded to expanded (C1)",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(panel),
        Some(&AccordionAnim::Expanding),
        "pressing the pip must drive the matching stat panel's accordion toward Expanding (C1)",
    );

    // Advance the lerp (the harness `FixedTimesteps(1)` steps `Time` a fixed delta per update, so a
    // few updates move the height-lerp measurably). The panel must actually OPEN — both the
    // progress parameter and the written `Node.height` move strictly off zero.
    for _ in 0..6 {
        app.update();
    }

    let progress = app
        .world()
        .get::<AccordionProgress>(panel)
        .map_or(0.0, |p| **p);
    assert!(
        progress > 0.0,
        "the shared drive_accordions lerp must advance the panel's AccordionProgress off zero \
         (C1: the panel must be an AccordionContent the height lerp iterates), got {progress}",
    );
    let height = panel_height_vh(&app, panel).unwrap_or(0.0);
    assert!(
        height > 0.0,
        "the lerp must write a growing Node.height onto the panel — the height growth is what \
         pushes the rows below it down (C1), got Vh({height})",
    );
}

/// C2: the expanded panel holds the EIGHT editable base-attribute numeric fields, each clamped via
/// a `NumericRange<f32>`.
///
/// Pin: a missing field (fewer than eight, or a missing attribute) fails the set assert; a field
/// without a clamp range fails the clamp assert.
#[test]
fn expanded_panel_has_eight_clamped_attribute_fields() {
    let mut app = editor_app();
    press_add_member(&mut app);

    let fields = attribute_fields_for_row(&mut app, 0);
    assert_eq!(
        fields.len(),
        8,
        "the expanded panel must spawn one numeric field per editable base attribute (C2)",
    );
    // All eight distinct attributes are present.
    for attribute in BaseAttribute::ALL {
        assert!(
            fields.iter().any(|(_, kind)| *kind == attribute),
            "the panel must have a field for {attribute:?} (C2)",
        );
    }

    // Each field carries a NumericRange<f32> that actually clamps out-of-range input (C2).
    for (field, attribute) in fields {
        let range = app.world().get::<NumericRange<f32>>(field).copied();
        assert!(
            range.is_some(),
            "an attribute field for {attribute:?} must carry its NumericRange<f32> clamp (C2)",
        );
        // Default range is degenerate; the assert above already failed if the real range is absent.
        let range = range.unwrap_or_else(|| NumericRange::new(0.0_f32, 0.0_f32));
        // The clamp returns EXACTLY the bound (same float), so compare bits to dodge the f32
        // strict-comparison lint — this is an exact-equality assertion, not a tolerance one.
        let above = range.max() + 1_000.0;
        assert_eq!(
            range.clamp(above).to_bits(),
            range.max().to_bits(),
            "the {attribute:?} field's range must clamp an above-max value down to max (C2)",
        );
        let below = range.min() - 1_000.0;
        assert_eq!(
            range.clamp(below).to_bits(),
            range.min().to_bits(),
            "the {attribute:?} field's range must clamp a below-min value up to min (C2)",
        );
    }
}

/// C3: a real `NumericFieldCommitted<f32>` on a member's attribute field updates that member's
/// attribute AND every displayed derived stat equals the GTW-384 `derive_stats` output for the
/// member's NEW attributes — proving the displayed value == pipeline(attrs).
///
/// Pin-discriminating: dropping the recompute leaves the OLD derived text (which differs, asserted
/// via the precondition), and using STALE attributes (deriving before the set) would also fail the
/// equality. The test computes its expected through the SAME `derive_stats` pipeline + tuning the
/// production code uses — it does NOT reimplement the derivation.
#[test]
fn editing_attribute_recomputes_derived_to_pipeline_output() {
    let mut app = editor_app();
    press_add_member(&mut app);

    // A fresh member's attributes are all 0.0; its derived stats are the pipeline output for the
    // all-zero attribute record. Edit Grit to a value that visibly moves HP / Wounds / Morale.
    let grit_field = {
        let fields = attribute_fields_for_row(&mut app, 0);
        fields
            .into_iter()
            .find(|(_, attribute)| *attribute == BaseAttribute::Grit)
            .map_or(Entity::PLACEHOLDER, |(entity, _)| entity)
    };

    // Precondition: HP currently reads the all-zero derivation (so the post-edit value differs).
    let zero_stats = derive_stats(&attributes_all(0.0), &test_tuning());
    assert_eq!(
        derived_text(&mut app, 0, DerivedStat::Hp).as_deref(),
        Some(expected_derived(&zero_stats, DerivedStat::Hp).as_str()),
        "precondition: a fresh member's HP display reads the all-zero pipeline output",
    );

    // Commit a new Grit via the REAL numeric-field commit message the widget raises.
    let new_grit = 8.0_f32;
    app.world_mut().write_message(NumericFieldCommitted::new(
        grit_field,
        CommittedNumericValue::new(new_grit),
    ));
    app.update();

    // The model attribute updated (C3) — compare bits (exact-equality, dodges the f32 lint).
    let model_grit = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| m.attribute(BaseAttribute::Grit)));
    assert_eq!(
        model_grit.map(f32::to_bits),
        Some(new_grit.to_bits()),
        "the commit must set THAT member's Grit attribute in the model (C3)",
    );

    // The expected derived stats come from the SAME pipeline + tuning the production code uses.
    let mut new_attrs = attributes_all(0.0);
    new_attrs.grit = Grit::new(new_grit);
    let expected = derive_stats(&new_attrs, &test_tuning());

    // EVERY displayed derived stat equals the pipeline output for the new attributes (C3).
    for stat in DerivedStat::ALL {
        assert_eq!(
            derived_text(&mut app, 0, stat).as_deref(),
            Some(expected_derived(&expected, stat).as_str()),
            "the displayed {stat:?} must equal the GTW-384 pipeline output for the live attributes \
             (C3: displayed == pipeline(attrs))",
        );
    }

    // The discriminator: HP actually MOVED off its all-zero value (so the assert is not vacuous).
    assert_ne!(
        expected_derived(&expected, DerivedStat::Hp),
        expected_derived(&zero_stats, DerivedStat::Hp),
        "the edit must change the derived HP (else the recompute assert would be vacuous)",
    );
}

/// All descendant entities of `root` (depth-first, `root` excluded) — the panel's full subtree, so
/// the layout-guard can confirm every stat line is REACHABLE under the panel content (C3).
fn descendants(app: &App, root: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = app.world().get::<Children>(entity) {
            for &child in children {
                out.push(child);
                stack.push(child);
            }
        }
    }
    out
}

/// The line-marker entities of kind `M` (an [`AttributeField`] / [`DerivedStatText`] marker) that
/// are descendants of `root` — i.e. laid out UNDER the panel content, reachable (C3).
fn marked_descendants<M: Component>(app: &App, root: Entity) -> Vec<Entity> {
    descendants(app, root)
        .into_iter()
        .filter(|&entity| app.world().get::<M>(entity).is_some())
        .collect()
}

/// GTW-428 layout-guard (the live-feature defect fix): after the pip opens the row-0 stat panel and
/// the accordion lerp SETTLES, ALL SIXTEEN stat lines (eight `AttributeField` + eight
/// `DerivedStatText`) are laid out and reachable under the panel content, AND the rest-open panel
/// sizes its height to FIT its content rather than clipping to a fixed `Vh` ceiling (the old shared
/// 18vh default — and the round-1 fixed 38vh — both clipped the taller column's bottom lines).
///
/// Concretely it asserts three things the pre-fix code fails:
///
/// 1. The settled panel's height is [`Val::Auto`] — the CONTENT-FIT rest height the generalized
///    accordion switches an [`AccordionContentFit`] section to once it settles fully open, so the
///    panel sizes to its exact content regardless of font metrics. The pre-fix code (and the
///    round-1 fixed-`Vh` attempt) settled to a `Val::Vh` ceiling and fails this `is Auto` assert.
/// 2. The panel lays its lines in EXACTLY TWO columns, each a direct child holding eight of the
///    sixteen lines (the compact 2-column layout). Pre-fix the sixteen lines hang in a single
///    column and the two-equal-columns assert fails.
/// 3. All eight `AttributeField` AND all eight `DerivedStatText` markers are reachable descendants
///    of the panel content (none orphaned by the restructure).
///
/// Pin-discriminating: reverting to a fixed-`Vh` settled height (dropping `AccordionContentFit`)
/// fails assert 1; reverting the 2-column split (back to one column of sixteen) fails assert 2;
/// dropping any line fails assert 3. The existing GTW-428 tests
/// (`pip_press_drives_the_accordion_target`, `expanded_panel_has_eight_clamped_attribute_fields`,
/// `editing_attribute_recomputes_derived_to_pipeline_output`) and the GTW-416 accordion tests stay
/// green — they assert behavior this fix preserves.
#[test]
fn expanded_panel_fits_all_sixteen_lines_above_the_old_clip() {
    let mut app = editor_app();
    press_add_member(&mut app);

    let pip = control_for_row::<ExpandPip>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    let panel = control_for_row::<MemberStatPanel>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    // Open the panel (the headless idiom — set Pressed then update), then let the lerp settle.
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(pip) {
        *interaction = Interaction::Pressed;
    }
    app.update();
    // Settle the lerp: the harness steps `Time` a fixed delta per update, so a generous run reaches
    // the per-instance expanded target and snaps to it.
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        app.world().get::<AccordionAnim>(panel),
        Some(&AccordionAnim::Expanded),
        "the panel must settle fully open (Expanded) before the layout is asserted",
    );

    // Assert 1 — the settled panel switched to a CONTENT-FIT (`Val::Auto`) height, so it sizes to
    // its exact content rather than clipping to a fixed `Vh` ceiling (the old 18vh default, and the
    // round-1 fixed 38vh, both clipped the taller column's bottom lines at the live window size).
    assert!(
        panel_height_is_auto(&app, panel),
        "the settled open panel must adopt a content-fit Val::Auto height (the AccordionContentFit \
         rest behavior) so all sixteen lines fit regardless of font metrics — got height {:?}",
        app.world().get::<Node>(panel).map(|node| node.height),
    );

    // Assert 2 — the panel holds EXACTLY two columns, each a direct child.
    let columns: Vec<Entity> = app
        .world()
        .get::<Children>(panel)
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default();
    assert_eq!(
        columns.len(),
        2,
        "the panel must lay its sixteen lines in EXACTLY two columns (the compact 2-column layout); \
         got {} direct children",
        columns.len(),
    );
    // Each column carries eight of the sixteen lines (8 attributes in one, 8 derived in the other).
    let mut per_column_line_counts: Vec<usize> = columns
        .iter()
        .map(|&column| {
            let attrs = marked_descendants::<AttributeField>(&app, column).len();
            let derived = marked_descendants::<DerivedStatText>(&app, column).len();
            attrs + derived
        })
        .collect();
    per_column_line_counts.sort_unstable();
    assert_eq!(
        per_column_line_counts,
        vec![8, 8],
        "each of the two columns must hold eight of the sixteen stat lines (got {per_column_line_counts:?})",
    );

    // Assert 3 — all sixteen line markers are reachable descendants of the panel content.
    let attribute_lines = marked_descendants::<AttributeField>(&app, panel);
    let derived_lines = marked_descendants::<DerivedStatText>(&app, panel);
    assert_eq!(
        attribute_lines.len(),
        8,
        "all eight editable AttributeField lines must be laid out under the open panel (C3); got {}",
        attribute_lines.len(),
    );
    assert_eq!(
        derived_lines.len(),
        8,
        "all eight readonly DerivedStatText lines must be laid out under the open panel (C3); got {}",
        derived_lines.len(),
    );
}

/// A [`GangerAttributes`] record with every attribute at `value` — the editor seeds a fresh member
/// at `0.0`, so this builds the matching expected-derivation input.
const fn attributes_all(value: f32) -> GangerAttributes {
    GangerAttributes {
        speed:     Speed::new(value),
        aim:       Aim::new(value),
        strength:  Strength::new(value),
        toughness: Toughness::new(value),
        reflexes:  Reflexes::new(value),
        cool:      Cool::new(value),
        grit:      Grit::new(value),
        luck:      Luck::new(value),
    }
}
