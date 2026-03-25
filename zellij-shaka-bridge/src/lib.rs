use std::collections::BTreeMap;

use zellij_tile::ZellijPlugin;
use zellij_tile::prelude::*;
use zellij_tile::register_plugin;
use zellij_tile::shim::report_panic;

#[derive(Debug, Default)]
pub struct ShakaBridge;

impl zellij_tile::ZellijPlugin for ShakaBridge {
    fn update(&mut self, event: zellij_tile::prelude::Event) -> bool {
        let Event::TabUpdate(_tab_infos) = event else {
            return false;
        };

        web_request(
            "https://localhost:5050/data",
            HttpVerb::Post,
            BTreeMap::from([("User-Agent".to_string(), "Zellij-Plugin".to_string())]),
            vec![],          // Body
            BTreeMap::new(), // Context
        );

        false
    }
}

register_plugin!(ShakaBridge);
