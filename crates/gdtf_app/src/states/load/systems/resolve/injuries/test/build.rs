use gdtf_battle_sim::{
    armor::{BodyPart, InjuryCategory},
    injuries::{DamageContext, InjuryName},
    rng::{BattleSeed, InjuryRng},
    severity::Severity,
};

use super::support::{add_def, add_folder, add_weighting, app, build, injury_def, weighting};

#[test]
fn builds_registry_and_tables_keyed_by_stem() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", InjuryCategory::Head, Severity::Critical) else {
        return;
    };
    let Some(w) = weighting(InjuryCategory::Head, &[], &[], &[("lost_eye", 4)]) else {
        return;
    };
    let def_handle = add_def(&mut app, "content/injuries/head/lost_eye.injury.ron", eye);
    let w_handle = add_weighting(&mut app, "content/injuries/weighting/head.weighting.ron", w);
    let folder = add_folder(&mut app, &[def_handle.untyped(), w_handle.untyped()]);

    let built = build(&app, &folder);
    assert!(
        built.is_some(),
        "build must succeed once both assets are in their collections",
    );
    let Some((registry, tables)) = built else {
        return;
    };

    let key = InjuryName::new("lost_eye".to_owned());
    assert!(registry.contains(&key), "lost_eye must resolve by stem");
    assert_eq!(registry.len(), 1, "exactly one injury registered");
    let bucket = tables.table(BodyPart::Head, DamageContext::Ranged, Severity::Critical);
    assert!(
        bucket.is_some_and(|t| t.iter().any(|r| r.injury == key)),
        "the Critical/Head bucket must hold lost_eye",
    );
    assert_eq!(tables.len(), 1, "only the non-empty bucket is tabled");
}

#[test]
fn canonical_sort_makes_the_seeded_pick_order_independent() {
    let pick = |entries_first: bool| -> Option<InjuryName> {
        let mut app = app();
        let a = injury_def("Alpha", InjuryCategory::Torso, Severity::Major)?;
        let b = injury_def("Bravo", InjuryCategory::Torso, Severity::Major)?;
        let a_h = add_def(&mut app, "content/injuries/torso/alpha.injury.ron", a);
        let b_h = add_def(&mut app, "content/injuries/torso/bravo.injury.ron", b);
        let rows: &[(&str, u32)] = if entries_first {
            &[("alpha", 3), ("bravo", 7)]
        } else {
            &[("bravo", 7), ("alpha", 3)]
        };
        let w = weighting(InjuryCategory::Torso, &[], rows, &[])?;
        let w_h = add_weighting(
            &mut app,
            "content/injuries/weighting/torso.weighting.ron",
            w,
        );
        let folder = add_folder(&mut app, &[a_h.untyped(), b_h.untyped(), w_h.untyped()]);
        let (_registry, tables) = build(&app, &folder)?;
        let table = tables.table(BodyPart::Torso, DamageContext::Ranged, Severity::Major)?;

        let total: u32 = table.iter().map(|r| *r.weight).sum();
        let mut rng = InjuryRng::from_root(BattleSeed::new(0xDEAD_BEEF));
        let draw: u32 = rng.random_range(0..total);
        let mut cumulative = 0u32;
        table.iter().find_map(|row| {
            cumulative += *row.weight;
            (draw < cumulative).then(|| row.injury.clone())
        })
    };

    let one = pick(true);
    let two = pick(false);
    assert!(one.is_some(), "the pick must resolve to an injury");
    assert_eq!(
        one, two,
        "the canonical sort must make the seeded pick identical regardless of authored \
         (folder-enumeration) order",
    );
}
