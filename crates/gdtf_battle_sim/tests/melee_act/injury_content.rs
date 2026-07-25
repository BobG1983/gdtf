//! The GTW-821 injury fixture: the per-context injury content, the severity-edge forcing,
//! and the `InjuryInflicted` recorder the melee-injury tests read.

use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, ResMut, Resource},
};
use gdtf_battle_sim::{
    acts::InjuryInflicted,
    armor::InjuryCategory,
    injuries::{
        DamageContext, InflictedInjuries, InjuryDef, InjuryName, InjuryRegistry, InjuryTables,
        InjuryWeight, InspectText, LogText, PopupText, PostHeal, WeightedInjuryEntry,
        WeightedInjuryTable,
    },
    severity::Severity,
    tuning::{CombatTuning, SeverityEdge, SeverityEdges, ViewRange},
};

use super::harness::TEST_VIEW_RANGE;

/// The name only the **melee** weighting tables can roll — a strike that sampled the ranged
/// or fall table could never produce it, so asserting on it proves the melee context reached
/// the roll.
pub(crate) const MELEE_ONLY: &str = "melee_only_gouged_eye";

/// The name only the **ranged / fall** weighting tables can roll — the decoy. Seeing it means
/// the melee path sampled the wrong per-source table.
pub(crate) const OTHER_ONLY: &str = "other_only_grazed_scalp";

/// A minimal named injury def — no effects (the roll only needs a resolvable key) at the
/// `Minor` severity, in `category`.
fn def(name: &InjuryName, category: InjuryCategory) -> InjuryDef {
    InjuryDef {
        name: name.clone(),
        category,
        severity: Severity::Minor,
        popup_text: PopupText::new("Hurt!".to_owned()),
        log_text: LogText::new("hurt".to_owned()),
        inspect_text: InspectText::new("A test injury.".to_owned()),
        effects: Vec::new(),
        post_heal: PostHeal::Deferred,
    }
}

/// A one-row weighted bucket naming `name`.
fn bucket(name: &InjuryName) -> WeightedInjuryTable {
    WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(
        name.clone(),
        InjuryWeight::new(1),
    )])
}

/// Install per-CONTEXT injury content: every category's `Minor`/`Major`/`Critical` bucket is
/// authored under ALL THREE contexts, but the MELEE buckets name [`MELEE_ONLY`] while the
/// ranged and fall buckets name [`OTHER_ONLY`].
///
/// This is what makes the melee-injury tests discriminating: a melee strike that sampled the
/// ranged or fall per-source table would roll the decoy, not the melee key — the assertion
/// names the key, so a context mix-up FAILS rather than passing on "some injury rolled".
/// (Content is authored here, in the test; no shipped weight magnitude is asserted.)
pub(crate) fn install_per_context_injury_content(app: &mut App) {
    let melee = InjuryName::new(MELEE_ONLY.to_owned());
    let other = InjuryName::new(OTHER_ONLY.to_owned());
    app.insert_resource(InjuryRegistry::new([
        (melee.clone(), def(&melee, InjuryCategory::Head)),
        (other.clone(), def(&other, InjuryCategory::Head)),
    ]));
    let mut rows = Vec::new();
    for category in InjuryCategory::ALL {
        for severity in [Severity::Minor, Severity::Major, Severity::Critical] {
            rows.push(((category, DamageContext::Melee, severity), bucket(&melee)));
            rows.push(((category, DamageContext::Ranged, severity), bucket(&other)));
            rows.push(((category, DamageContext::Fall, severity), bucket(&other)));
        }
    }
    app.insert_resource(InjuryTables::new(rows));
}

/// A [`CombatTuning`] whose §6 severity edges FORCE every connecting hit into `edges`'
/// intended bucket band, leaving the rest of the tuning at its defaults.
///
/// Tuning is authored per-test here (never a shipped magnitude assertion): moving the four
/// ascending edges far outside the reachable score range is what makes "this wound is
/// non-graze / is a graze / is fatal" a deterministic property of the fixture rather than a
/// lucky roll.
fn tuning_with_edges(edges: SeverityEdges) -> CombatTuning {
    let mut tuning = CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    };
    tuning.severity_scaling.edges = edges;
    tuning
}

/// Tuning under which every connecting hit lands in the WOUND band (`Minor` / `Major` /
/// `Critical`): the graze edge sits far below any reachable score and the fatal edge far
/// above it.
pub(crate) fn wounding_tuning() -> CombatTuning {
    tuning_with_edges(SeverityEdges {
        e0: SeverityEdge::new(-1.0e6),
        e1: SeverityEdge::new(-1.0e5),
        e2: SeverityEdge::new(1.0e5),
        e3: SeverityEdge::new(1.0e6),
    })
}

/// Tuning under which every connecting hit is a GRAZE ([`Severity::None`]) — the graze edge
/// sits far above any reachable score.
pub(crate) fn grazing_tuning() -> CombatTuning {
    tuning_with_edges(SeverityEdges {
        e0: SeverityEdge::new(1.0e6),
        e1: SeverityEdge::new(2.0e6),
        e2: SeverityEdge::new(3.0e6),
        e3: SeverityEdge::new(4.0e6),
    })
}

/// Tuning under which every connecting hit is FATAL — all four edges sit far below any
/// reachable score, so the score is always `≥ e3`.
pub(crate) fn fatal_tuning() -> CombatTuning {
    tuning_with_edges(SeverityEdges {
        e0: SeverityEdge::new(-4.0e6),
        e1: SeverityEdge::new(-3.0e6),
        e2: SeverityEdge::new(-2.0e6),
        e3: SeverityEdge::new(-1.0e6),
    })
}

/// Every `InjuryInflicted` observed across the run — a test-local recorder, since a
/// `MessageReader` sees only the current+previous update (the `MeleeLog` precedent).
#[derive(Resource, Default)]
pub(crate) struct InjuryLog {
    /// One `(target, injury name)` per `InjuryInflicted` emitted.
    entries: Vec<(Entity, String)>,
}

/// Drain `InjuryInflicted` into the recorder (its own cursor — the `apply_injury` boundary
/// still sees every message).
fn record_injuries(mut reader: MessageReader<InjuryInflicted>, mut log: ResMut<InjuryLog>) {
    for message in reader.read() {
        log.entries
            .push((message.target, (*message.gained.name).clone()));
    }
}

/// Add the `InjuryInflicted` recorder (after the sim plugin, so the buffer exists).
pub(crate) fn with_injury_log(app: &mut App) {
    app.init_resource::<InjuryLog>();
    app.add_systems(Update, record_injuries);
}

/// The injury names recorded for `entity` across the run.
pub(crate) fn injuries_emitted_for(app: &App, entity: Entity) -> Vec<String> {
    app.world()
        .get_resource::<InjuryLog>()
        .map(|log| {
            log.entries
                .iter()
                .filter(|(target, _)| *target == entity)
                .map(|(_, name)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The injury names on `entity`'s durable ledger (what `apply_injury` folded on).
pub(crate) fn ledger_names(app: &App, entity: Entity) -> Vec<String> {
    app.world()
        .get::<InflictedInjuries>(entity)
        .map(|ledger| {
            ledger
                .gained()
                .iter()
                .map(|gained| (*gained.name).clone())
                .collect()
        })
        .unwrap_or_default()
}
