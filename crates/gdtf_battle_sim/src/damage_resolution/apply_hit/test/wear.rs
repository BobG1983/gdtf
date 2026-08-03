use super::support::*;

#[test]
fn apply_hit_wears_the_struck_piece_and_can_break_it() {
    let tuning = CombatTuning::default();
    let part = BodyPart::RightArm;

    let ganger1 = a_ganger();
    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    let mut integrity = worn_piece_integrity(20);
    let mut inflicted = InflictedWounds::default();
    let before = *integrity;
    {
        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: Some(&mut integrity),
            inflicted: &mut inflicted,
        };
        let outcome = apply_hit(target, &hit(1, 6), Severity::Minor, part, ganger1, &tuning);
        assert_eq!(
            outcome,
            ArmorWearOutcome::Damaged(ArmorDamaged::new(ganger1, part, IntegrityWear::new(6))),
            "a sub-fatal wear on a sturdy piece must not break it — it must report Damaged(delta=6)",
        );
    }
    assert_eq!(
        *integrity,
        before - 6,
        "apply_hit must drop the struck piece entity's integrity by exactly the hit's wear",
    );

    let ganger = a_ganger();
    let mut hp2 = Hp::new(50);
    let mut wounds2 = Wounds::new(9);
    let mut life2 = LifeState::Alive;
    let mut integrity2 = worn_piece_integrity(1); 
    let mut inflicted2 = InflictedWounds::default();
    let outcome = {
        let target = GangerHitTarget {
            hp:        &mut hp2,
            wounds:    &mut wounds2,
            life:      &mut life2,
            integrity: Some(&mut integrity2),
            inflicted: &mut inflicted2,
        };
        apply_hit(target, &hit(1, 5), Severity::Minor, part, ganger, &tuning)
    };
    assert_eq!(
        outcome,
        ArmorWearOutcome::Broke(ArmorBroken::new(ganger, part)),
        "a high-wear hit crossing a near-broken piece to ≤ 0 must return Broke(ArmorBroken)",
    );
    assert!(
        *integrity2 <= 0,
        "the struck piece must be broken (≤ 0) after the crossing hit",
    );
}

/// a buffered `#[derive(Message)]`, written through a real `MessageWriter` at the
#[test]
fn apply_hit_armor_broken_flows_through_a_message_buffer() {
    use bevy::prelude::{IntoScheduleConfigs, MessageReader, MessageWriter, ResMut, Resource};

            #[derive(Resource, Default)]
    struct Captured(Vec<ArmorBroken>);

    let ganger = a_ganger();
    let part = BodyPart::Torso;

    let produce = move |mut writer: MessageWriter<ArmorBroken>| {
        let tuning = CombatTuning::default();
        let mut hp = Hp::new(30);
        let mut wounds = Wounds::new(6);
        let mut life = LifeState::Alive;
        let mut integrity = worn_piece_integrity(1);
        let mut inflicted = InflictedWounds::default();
        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: Some(&mut integrity),
            inflicted: &mut inflicted,
        };
        if let ArmorWearOutcome::Broke(broke) =
            apply_hit(target, &hit(1, 5), Severity::Minor, part, ganger, &tuning)
        {
            writer.write(broke);
        }
    };

    let consume = |mut reader: MessageReader<ArmorBroken>, mut captured: ResMut<Captured>| {
        for broke in reader.read() {
            captured.0.push(*broke);
        }
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<ArmorBroken>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (produce, consume).chain());

    app.update();

    let captured = app
        .world()
        .get_resource::<Captured>()
        .map_or_else(Vec::new, |c| c.0.clone());

    assert_eq!(
        captured.len(),
        1,
        "apply_hit's ArmorBroken must reach the message buffer once"
    );
    assert_eq!(
        captured.first(),
        Some(&ArmorBroken::new(ganger, part)),
        "the buffered ArmorBroken must carry the struck ganger Entity + BodyPart",
    );
}

#[test]
fn wound_cost_helper_maps_tiers_and_newtype_derefs() {
    let costs = WoundCosts::default();
    assert_eq!(*wound_cost(Severity::None, costs), 0);
    assert_eq!(*wound_cost(Severity::Fatal, costs), 0);
    let minor = *wound_cost(Severity::Minor, costs);
    let major = *wound_cost(Severity::Major, costs);
    let critical = *wound_cost(Severity::Critical, costs);
    assert!(
        minor < major,
        "Minor cost must be < Major: {minor} >= {major}"
    );
    assert!(
        major < critical,
        "Major cost must be < Critical: {major} >= {critical}"
    );
    assert_eq!(*WoundCost::new(7), 7u8);
}
