use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

#[derive(Clone, Debug)]
pub(crate) struct ThemeOption {
    label: String,
    key:   ThemeUuid,
}

impl ThemeOption {
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) const fn key(&self) -> ThemeUuid {
        self.key
    }
}

pub(crate) fn theme_options(themes: Option<&UuidThemeRegistry>) -> Vec<ThemeOption> {
    let Some(themes) = themes else {
        return Vec::new();
    };
    let mut options: Vec<ThemeOption> = themes
        .defs()
        .map(|(key, def)| ThemeOption {
            label: (*def.display_name).clone(),
            key:   *key,
        })
        .collect();
    options.sort_by(|a, b| a.label.cmp(&b.label));
    options
}
