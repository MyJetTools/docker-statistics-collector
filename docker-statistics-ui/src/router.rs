//use dioxus_router_macro::Routable;
use crate::Home;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(dioxus_router_macro::Routable, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AppRoute {
    #[route("/")]
    Home,
}
