use dioxus_utils::js::GlobalAppSettings;

const STORAGE_KEY: &str = "dockerscope:prefs";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn data_attr(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    fn parse(s: &str) -> Self {
        if s.trim() == "light" {
            Theme::Light
        } else {
            Theme::Dark
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prefs {
    pub theme: Theme,
}

impl Prefs {
    pub fn load() -> Self {
        let raw = GlobalAppSettings::get_local_storage().get(STORAGE_KEY);
        let theme = match raw {
            Some(s) => Theme::parse(&s),
            // Nothing stored yet → follow the OS preference so the app doesn't
            // blast white-on-black in a light-mode session (or vice-versa).
            None => system_theme().unwrap_or(Theme::Dark),
        };
        Self { theme }
    }

    pub fn save(&self) {
        GlobalAppSettings::get_local_storage().set(STORAGE_KEY, self.theme.data_attr());
    }
}

fn system_theme() -> Option<Theme> {
    let window = web_sys::window()?;
    let mql = window.match_media("(prefers-color-scheme: dark)").ok().flatten()?;
    Some(if mql.matches() { Theme::Dark } else { Theme::Light })
}
