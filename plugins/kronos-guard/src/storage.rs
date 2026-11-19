use crate::host;
use crate::models::{BlockLogEntry, GuardConfig, GuardStore};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};

const CONFIG_PATH: &str = "data/config.json";
const STORE_PATH: &str = "data/guard_store.json";
const BLOCKLOG_PATH: &str = "data/guard_blocklog.json";
const RANKS_BRIDGE_PATH: &str = "data/kronos_ranks_bridge.json";
const EMBEDDED_DEFAULT_CONFIG: &str = include_str!("../data/config.json");

#[derive(Deserialize)]
struct BridgeEntry {
    primary_rank: String,
}

pub fn default_config() -> GuardConfig {
    serde_json::from_str::<GuardConfig>(EMBEDDED_DEFAULT_CONFIG)
        .expect("embedded kronos-guard data/config.json must be valid")
}

pub fn load_config() -> GuardConfig {
    if let Some(raw) = host::read_file(CONFIG_PATH) {
        if let Ok(cfg) = serde_json::from_str::<GuardConfig>(&raw) {
            return cfg;
        }
    }
    default_config()
}

/// Reads the live primary rank from `kronos-ranks` cross-plugin bridge if available.
pub fn resolve_rank_from_bridge(player_name: &str) -> Option<String> {
    let raw = host::read_file(RANKS_BRIDGE_PATH)?;
    let map = serde_json::from_str::<HashMap<String, BridgeEntry>>(&raw).ok()?;
    map.get(&player_name.trim().to_lowercase())
        .map(|e| e.primary_rank.clone())
}

pub fn load_store() -> GuardStore {
    if let Some(raw) = host::read_file(STORE_PATH) {
        if let Ok(mut store) = serde_json::from_str::<GuardStore>(&raw) {
            if store.next_punishment_id == 0 {
                store.next_punishment_id = 1;
            }
            if store.next_report_id == 0 {
                store.next_report_id = 1;
            }
            if store.next_rollback_id == 0 {
                store.next_rollback_id = 1;
            }
            return store;
        }
    }
    GuardStore {
        next_punishment_id: 1,
        next_report_id: 1,
        next_rollback_id: 1,
        ..GuardStore::default()
    }
}

pub fn save_store_atomic(store: &GuardStore) -> bool {
    match serde_json::to_string_pretty(store) {
        Ok(json) => host::write_file_atomic(STORE_PATH, &json),
        Err(_) => false,
    }
}

pub fn load_block_log() -> VecDeque<BlockLogEntry> {
    if let Some(raw) = host::read_file(BLOCKLOG_PATH) {
        if let Ok(entries) = serde_json::from_str::<VecDeque<BlockLogEntry>>(&raw) {
            return entries;
        }
    }
    VecDeque::new()
}

pub fn save_block_log_atomic(entries: &VecDeque<BlockLogEntry>) -> bool {
    match serde_json::to_string(entries) {
        Ok(json) => host::write_file_atomic(BLOCKLOG_PATH, &json),
        Err(_) => false,
    }
}
