use crate::models::{EssentialsConfig, EssentialsStore};
use log::{info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const DATA_DIR: &str = "data/kronos-essentials";
const CONFIG_PATH: &str = "data/kronos-essentials/config.json";
const STORE_PATH: &str = "data/kronos-essentials/store.json";
const STORE_TMP_PATH: &str = "data/kronos-essentials/store.json.tmp";
const RANKS_BRIDGE_PATH: &str = "data/kronos_ranks_bridge.json";
const EMBEDDED_DEFAULT_CONFIG: &str = include_str!("../data/config.json");

#[derive(Deserialize)]
struct BridgeEntry {
    primary_rank: String,
    donate_rank: String,
    max_homes: u32,
}

pub fn default_config() -> EssentialsConfig {
    serde_json::from_str::<EssentialsConfig>(EMBEDDED_DEFAULT_CONFIG)
        .expect("embedded kronos-essentials data/config.json must be valid")
}

/// Resolves player's effective rank from `kronos-ranks` cross-plugin bridge (preserving dual-track donate+staff homes).
pub fn resolve_rank_from_bridge(config: &EssentialsConfig, player_name: &str) -> Option<String> {
    let raw = fs::read_to_string(RANKS_BRIDGE_PATH).ok()?;
    let map = serde_json::from_str::<HashMap<String, BridgeEntry>>(&raw).ok()?;
    let entry = map.get(&player_name.trim().to_lowercase())?;
    let primary_prof = config.rank_for(&entry.primary_rank);
    let donate_prof = config.rank_for(&entry.donate_rank);
    if primary_prof.can_set_spawn || primary_prof.max_homes >= entry.max_homes {
        Some(entry.primary_rank.clone())
    } else if donate_prof.max_homes > primary_prof.max_homes {
        Some(entry.donate_rank.clone())
    } else {
        Some(entry.primary_rank.clone())
    }
}

pub fn load_config() -> EssentialsConfig {
    if Path::new(CONFIG_PATH).exists() {
        match fs::read_to_string(CONFIG_PATH)
            .ok()
            .and_then(|raw| serde_json::from_str::<EssentialsConfig>(&raw).ok())
        {
            Some(cfg) => {
                info!("Loaded KRONOS Essentials v2 config from {}", CONFIG_PATH);
                return cfg;
            }
            None => warn!("Invalid config.json, using built-in defaults"),
        }
    }

    let cfg = default_config();
    let _ = fs::create_dir_all(DATA_DIR);
    if let Ok(pretty) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(CONFIG_PATH, pretty);
    }
    cfg
}

pub fn load_store() -> EssentialsStore {
    if Path::new(STORE_PATH).exists() {
        if let Ok(raw) = fs::read_to_string(STORE_PATH) {
            if let Ok(store) = serde_json::from_str::<EssentialsStore>(&raw) {
                info!("Loaded KRONOS Essentials store ({} players)", store.players.len());
                return store;
            }
        }
    }
    EssentialsStore::default()
}

/// Atomic crash-safe write: writes to store.json.tmp first, then atomically renames to store.json.
pub fn save_store_atomic(store: &EssentialsStore) {
    let _ = fs::create_dir_all(DATA_DIR);
    if let Ok(pretty) = serde_json::to_string_pretty(store) {
        if let Err(err) = fs::write(STORE_TMP_PATH, pretty) {
            warn!("Failed writing temporary store file: {}", err);
            return;
        }
        if let Err(err) = fs::rename(STORE_TMP_PATH, STORE_PATH) {
            warn!("Failed atomic rename {} -> {}: {}", STORE_TMP_PATH, STORE_PATH, err);
        }
    }
}
