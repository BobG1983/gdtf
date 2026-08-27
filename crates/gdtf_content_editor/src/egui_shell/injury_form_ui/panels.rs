//! The INJURY tab's two sub-tabs: the def form and the weighting table.

use std::path::Path;

use bevy_egui::egui;
use gdtf_battle_sim::injuries::{InjuryRegistry, InjuryTables};

use super::{def_panel, weighting_panel};
use crate::{
    injury_form::{InjuryDraft, WeightingDraft},
    mode::InjurySubTab,
    save_record::LastSaveRecord,
};

#[cfg(test)]
mod test;

/// Everything the Injury sub-tabs draw from, as plain borrows a unit test can build.
pub(crate) struct InjuryPanelsCtx<'a> {
    pub(crate) sub_tab:   &'a mut InjurySubTab,
    pub(crate) draft:     Option<&'a mut InjuryDraft>,
    pub(crate) weighting: Option<&'a mut WeightingDraft>,
    pub(crate) injuries:  Option<&'a InjuryRegistry>,
    pub(crate) tables:    Option<&'a InjuryTables>,
    pub(crate) last_save: &'a mut LastSaveRecord,
    pub(crate) root:      Option<&'a Path>,
}

pub(crate) fn injury_panels(ui: &mut egui::Ui, ctx: &mut InjuryPanelsCtx<'_>) {
    ui.horizontal(|ui| {
        for tab in InjurySubTab::TAB_ORDER {
            ui.selectable_value(ctx.sub_tab, tab, tab.tab_label());
        }
    });
    ui.separator();

    match *ctx.sub_tab {
        InjurySubTab::Def => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(draft) = ctx.draft.as_deref_mut() {
                        def_panel(ui, draft);
                    }
                });
        }
        InjurySubTab::Tables => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(weighting) = ctx.weighting.as_deref_mut() {
                        weighting_panel(
                            ui,
                            weighting,
                            ctx.injuries,
                            ctx.tables,
                            ctx.last_save,
                            ctx.root,
                        );
                    }
                });
        }
    }
}
