use std::collections::BTreeMap;

use zellij_tile::ZellijPlugin;
use zellij_tile::prelude::*;
use zellij_tile::register_plugin;
use zellij_tile::shim::report_panic;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Default)]
pub struct ShakaBridge;

#[derive(Serialize, Deserialize, Debug)]
pub struct ActiveTabInfo {
    tab_index: usize,
}

impl zellij_tile::ZellijPlugin for ShakaBridge {
    fn update(&mut self, event: zellij_tile::prelude::Event) -> bool {
        let Event::TabUpdate(tab_infos) = event else {
            return false;
        };

        if let Some(active_tab) = tab_infos.iter().find(|t| t.active) {
            let active_tab_info = ActiveTabInfo {
                tab_index: active_tab.position,
            };

            let body = serde_json::to_vec(&active_tab_info).unwrap_or_default();

            web_request(
                "https://localhost:5050/data",
                HttpVerb::Post,
                BTreeMap::from([("User-Agent".to_string(), "Zellij-Plugin".to_string())]),
                body,            // Body
                BTreeMap::new(), // Context
            );
        }

        false
    }
}

register_plugin!(ShakaBridge);
