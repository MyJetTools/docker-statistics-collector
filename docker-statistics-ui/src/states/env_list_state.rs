use std::rc::Rc;

use dioxus_utils::{js::WebLocalStorage, DataState};

pub struct EnvListState {
    pub items: DataState<Vec<Rc<String>>>,
    selected_env: Option<Rc<String>>,
    storage: WebLocalStorage,
}

impl EnvListState {
    pub fn new() -> Self {
        Self {
            items: Default::default(),
            selected_env: None,
            storage: dioxus_utils::js::GlobalAppSettings::get_local_storage(),
        }
    }

    pub fn has_envs(&self) -> bool {
        self.items.try_unwrap_as_loaded().is_some()
    }

    pub fn get_selected_env(&self) -> Option<Rc<String>> {
        self.selected_env.clone()
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        let items: Vec<Rc<String>> = items.into_iter().map(|itm| Rc::new(itm)).collect();
        self.items.set_value(items);

        let selected_env = self.storage.get("env").unwrap_or_default();
        self.update_active_env(selected_env.as_str());
    }

    pub fn set_error(&mut self, error: String) {
        self.items.set_error(error);
    }

    fn update_active_env(&mut self, selected_env: &str) {
        if let Some(items) = self.items.try_unwrap_as_loaded() {
            if items.len() == 0 {
                return;
            }

            let index = items.iter().position(|itm| itm.as_str() == selected_env);

            match index {
                Some(index) => {
                    self.selected_env = Some(items[index].clone());
                }
                None => {
                    self.selected_env = items.first().cloned();
                }
            }
        }
    }

    pub fn set_active_env(&mut self, selected_env: &str) {
        if self.items.is_none() {
            panic!("Should net set active env before envs are loaded");
        }

        self.update_active_env(selected_env);

        if let Some(selected_env) = self.selected_env.as_ref() {
            self.storage.set("env", selected_env);
        }
    }
}
