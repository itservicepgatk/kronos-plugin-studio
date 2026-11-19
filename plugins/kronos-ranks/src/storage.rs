use crate::hierarchy;
use crate::host;
use crate::models::{BridgeRankSnapshot, RanksConfig, RanksStore};
use std::collections::HashMap;

const CONFIG_PATH: &str = "data/config.json";
const STORE_PATH: &str = "data/ranks_store.json";
const EMBEDDED_DEFAULT_CONFIG: &str = include_str!("../data/config.json");

pub fn default_config() -> RanksConfig {
    serde_json::from_str::<RanksConfig>(EMBEDDED_DEFAULT_CONFIG)
        .expect("embedded kronos-ranks data/config.json must be valid")
}

pub fn load_config() -> RanksConfig {
    if let Some(raw) = host::read_file(CONFIG_PATH) {
        if let Ok(cfg) = serde_json::from_str::<RanksConfig>(&raw) {
            if !cfg.ranks.is_empty() {
                return cfg;
            }
        }
    }
    default_config()
}

pub fn load_store() -> RanksStore {
    if let Some(raw) = host::read_file(STORE_PATH) {
        if let Ok(mut store) = serde_json::from_str::<RanksStore>(&raw) {
            if store.next_app_id == 0 {
                store.next_app_id = 1;
            }
            if store.next_audit_id == 0 {
                store.next_audit_id = 1;
            }
            return store;
        }
    }
    RanksStore {
        next_app_id: 1,
        next_audit_id: 1,
        ..RanksStore::default()
    }
}

pub fn save_store_atomic(store: &RanksStore) -> bool {
    match serde_json::to_string_pretty(store) {
        Ok(json) => host::write_file_atomic(STORE_PATH, &json),
        Err(_) => false,
    }
}

/// Exports a compact cross-plugin snapshot (`data/kronos_ranks_bridge.json`)
/// so `kronos-essentials`, `kronos-regions`, and `kronos-guard` stay in full sync with dual-track ranks.
pub fn export_bridge_snapshot(config: &RanksConfig, store: &RanksStore) -> bool {
    let mut map: HashMap<String, BridgeRankSnapshot> = HashMap::new();
    for (key, profile) in &store.profiles {
        let primary = hierarchy::primary_rank(config, profile);
        let eff_perks = hierarchy::effective_perks(config, profile);
        map.insert(
            key.clone(),
            BridgeRankSnapshot {
                primary_rank: primary.id.clone(),
                donate_rank: profile.donate_rank.clone(),
                staff_rank: profile.staff_rank.clone(),
                department: primary.department.code().to_string(),
                weight: primary.weight,
                prefix: primary.prefix.clone(),
                max_homes: eff_perks.max_homes,
                max_regions: eff_perks.max_regions,
                max_region_volume: eff_perks.max_region_volume,
            },
        );
    }
    match serde_json::to_string_pretty(&map) {
        Ok(json) => host::write_file_atomic(&config.bridge_export_path, &json),
        Err(_) => false,
    }
}
