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

pub(crate) const MELEE_ONLY: &str = "melee_only_gouged_eye";

pub(crate) const OTHER_ONLY: &str = "other_only_grazed_scalp";

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

fn bucket(name: &InjuryName) -> WeightedInjuryTable {
    WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(
        name.clone(),
        InjuryWeight::new(1),
    )])
}

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

fn tuning_with_edges(edges: SeverityEdges) -> CombatTuning {
    let mut tuning = CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    };
    tuning.severity_scaling.edges = edges;
    tuning
}

pub(crate) fn wounding_tuning() -> CombatTuning {
    tuning_with_edges(SeverityEdges {
        e0: SeverityEdge::new(-1.0e6),
        e1: SeverityEdge::new(-1.0e5),
        e2: SeverityEdge::new(1.0e5),
        e3: SeverityEdge::new(1.0e6),
    })
}

pub(crate) fn grazing_tuning() -> CombatTuning {
    tuning_with_edges(SeverityEdges {
        e0: SeverityEdge::new(1.0e6),
        e1: SeverityEdge::new(2.0e6),
        e2: SeverityEdge::new(3.0e6),
        e3: SeverityEdge::new(4.0e6),
    })
}

pub(crate) fn fatal_tuning() -> CombatTuning {
    tuning_with_edges(SeverityEdges {
        e0: SeverityEdge::new(-4.0e6),
        e1: SeverityEdge::new(-3.0e6),
        e2: SeverityEdge::new(-2.0e6),
        e3: SeverityEdge::new(-1.0e6),
    })
}

#[derive(Resource, Default)]
pub(crate) struct InjuryLog {
    entries: Vec<(Entity, String)>,
}

fn record_injuries(mut reader: MessageReader<InjuryInflicted>, mut log: ResMut<InjuryLog>) {
    for message in reader.read() {
        log.entries
            .push((message.target, (*message.gained.name).clone()));
    }
}

pub(crate) fn with_injury_log(app: &mut App) {
    app.init_resource::<InjuryLog>();
    app.add_systems(Update, record_injuries);
}

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
