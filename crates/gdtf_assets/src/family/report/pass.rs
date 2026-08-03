use bevy::{
    app::{App, Update},
    ecs::{
        resource::Resource,
        schedule::{
            IntoScheduleConfigs, SystemCondition, SystemSet,
            common_conditions::{not, resource_exists},
        },
        system::{Commands, Res, ScheduleSystem},
    },
    log::{info, warn},
};

use super::finding::ContentFinding;

#[derive(Resource, Debug, Default)]
pub struct ContentIntegrityReport(Vec<ContentFinding>);

impl ContentIntegrityReport {
        pub fn record(&mut self, finding: ContentFinding) {
        self.0.push(finding);
    }

        #[must_use]
    pub fn findings(&self) -> &[ContentFinding] {
        &self.0
    }

        #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.0.is_empty()
    }

        #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

        #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentValidationSet {
            Check,
            Publish,
}

#[derive(Resource, Debug, Default)]
struct ContentValidationPlumbing;

#[derive(Resource, Debug, Default)]
pub struct ContentChecksComplete;

#[derive(Resource, Debug, Default)]
pub struct ContentValidationDone;

pub fn mark_content_checks_complete(mut commands: Commands) {
    commands.insert_resource(ContentChecksComplete);
}

pub fn publish_content_integrity_report(
    report: Option<Res<ContentIntegrityReport>>,
    mut commands: Commands,
) {
    match report.as_deref() {
        Some(report) if !report.is_clean() => {
            let lines: Vec<String> = report
                .findings()
                .iter()
                .map(|finding| format!("  - {finding}"))
                .collect();
            warn!(
                "content reference contract: {} finding(s) at end of Load:\n{}",
                report.len(),
                lines.join("\n"),
            );
        }
        Some(_) => {
            info!("content reference contract: every authored reference resolves (0 findings)");
        }
        None => {}
    }
    commands.insert_resource(ContentValidationDone);
}

pub trait ContentValidationAppExt {
                                                        fn init_content_validation(&mut self) -> &mut Self;

                    fn register_reference_check<M>(
        &mut self,
        check: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;
}

impl ContentValidationAppExt for App {
    fn init_content_validation(&mut self) -> &mut Self {
        if self
            .world()
            .get_resource::<ContentValidationPlumbing>()
            .is_none()
        {
            self.insert_resource(ContentValidationPlumbing);
            self.init_resource::<ContentIntegrityReport>();
            self.configure_sets(
                Update,
                ContentValidationSet::Publish
                    .after(ContentValidationSet::Check)
                    .run_if(
                        resource_exists::<ContentChecksComplete>
                            .and_then(not(resource_exists::<ContentValidationDone>)),
                    ),
            );
            self.add_systems(
                Update,
                mark_content_checks_complete.in_set(ContentValidationSet::Check),
            );
            self.add_systems(
                Update,
                publish_content_integrity_report.in_set(ContentValidationSet::Publish),
            );
        }
        self
    }

    fn register_reference_check<M>(
        &mut self,
        check: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.init_content_validation();
        self.add_systems(Update, check.in_set(ContentValidationSet::Check));
        self
    }
}
