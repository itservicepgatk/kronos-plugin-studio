mod commands;
mod hierarchy;
mod host;
mod menu;
mod models;
mod storage;
mod tab_chat;

use log::info;
use models::{PlayerRankProfile, RanksConfig, RanksStore};
use std::collections::HashMap;
use std::slice;
use std::str;
use std::sync::Mutex;

struct PluginState {
    config: RanksConfig,
    store: RanksStore,
    /// lower_name -> player_id
    online_keys: HashMap<String, String>,
    dirty: bool,
    last_autosave_ms: u64,
    tick_counter: u64,
}

static STATE: Mutex<Option<PluginState>> = Mutex::new(None);

fn read_str<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    unsafe { str::from_utf8(slice::from_raw_parts(ptr, len)).unwrap_or("") }
}

#[no_mangle]
pub extern "C" fn init() -> i32 {
    let config = storage::load_config();
    let store = storage::load_store();
    let now = host::now_ms();
    storage::export_bridge_snapshot(&config, &store);

    let mut guard = STATE.lock().unwrap();
    *guard = Some(PluginState {
        config,
        store,
        online_keys: HashMap::new(),
        dirty: false,
        last_autosave_ms: now,
        tick_counter: 0,
    });
    info!("KRONOS Ranks & Hierarchy v1.0.0 initialized (20 ranks: Player + 10 Donate + 9 Staff Departments)");
    0
}

#[no_mangle]
pub extern "C" fn cleanup() {
    let mut guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        if state.dirty {
            storage::save_store_atomic(&state.store);
            storage::export_bridge_snapshot(&state.config, &state.store);
        }
    }
    *guard = None;
    info!("KRONOS Ranks saved state and unloaded.");
}

#[no_mangle]
pub extern "C" fn on_player_join(
    player_id_ptr: *const u8,
    player_id_len: usize,
    name_ptr: *const u8,
    name_len: usize,
) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let name = read_str(name_ptr, name_len);
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let display_name = if name.is_empty() { player_id } else { name };
        let key = hierarchy::normalize_key(display_name);
        state.store.id_to_name.insert(player_id.to_string(), display_name.to_string());
        state.online_keys.insert(key.clone(), player_id.to_string());

        let profile = state
            .store
            .profiles
            .entry(key)
            .or_insert_with(|| PlayerRankProfile::new(display_name, now))
            .clone();

        tab_chat::refresh_player_display(&state.config, player_id, &profile, state.online_keys.len());
        tab_chat::announce_join_flair(&state.config, player_id, &profile);
    }
}

#[no_mangle]
pub extern "C" fn on_player_leave(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if let Some(name) = state.store.id_to_name.get(player_id).cloned() {
            state.online_keys.remove(&hierarchy::normalize_key(&name));
        }
    }
}

/// 20-TPS tick: expires temporary ranks, refreshes tab online counts, and atomically saves + syncs cross-plugin bridge.
#[no_mangle]
pub extern "C" fn on_tick() {
    let mut guard = STATE.lock().unwrap();
    let Some(ref mut state) = *guard else {
        return;
    };
    state.tick_counter = state.tick_counter.wrapping_add(1);
    let now = host::now_ms();

    // Every 5 seconds: expire temporary donate/staff ranks
    if state.tick_counter % 100 == 0 {
        let expired = hierarchy::expire_temp_ranks(&mut state.store, &mut state.dirty, now);
        for (pkey, old_rank) in expired {
            if let (Some(pid), Some(prof)) = (state.online_keys.get(&pkey), state.store.profiles.get(&pkey)) {
                tab_chat::refresh_player_display(&state.config, pid, prof, state.online_keys.len().max(1));
                host::send_chat(
                    pid,
                    &format!("§eСрок действия временного статуса §6{} §eистёк.", old_rank),
                );
            }
        }
    }

    // Debounced atomic autosave + cross-plugin bridge sync
    if state.tick_counter % 20 == 0 && state.dirty {
        let interval_ms = state.config.autosave_interval_seconds.max(5) * 1000;
        if now.saturating_sub(state.last_autosave_ms) >= interval_ms {
            storage::save_store_atomic(&state.store);
            storage::export_bridge_snapshot(&state.config, &state.store);
            state.dirty = false;
            state.last_autosave_ms = now;
        }
    }
}

/// Formats and broadcasts chat message with rank prefix, suffix, and color codes.
#[no_mangle]
pub extern "C" fn on_player_chat(
    player_id_ptr: *const u8,
    player_id_len: usize,
    msg_ptr: *const u8,
    msg_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let message = read_str(msg_ptr, msg_len);
    let now = host::now_ms();

    let guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let name = state
            .store
            .id_to_name
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.to_string());
        let key = hierarchy::normalize_key(&name);
        let prof = state
            .store
            .profiles
            .get(&key)
            .cloned()
            .unwrap_or_else(|| PlayerRankProfile::new(&name, now));
        let formatted = tab_chat::format_chat_line(&state.config, &prof, message);
        host::broadcast_chat(&formatted);
        return false; // handled with formatted broadcast
    }
    true
}

#[no_mangle]
pub extern "C" fn on_player_command(
    player_id_ptr: *const u8,
    player_id_len: usize,
    cmd_ptr: *const u8,
    cmd_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let command = read_str(cmd_ptr, cmd_len);

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let handled = commands::handle_command(
            &state.config,
            &mut state.store,
            &state.online_keys,
            &mut state.dirty,
            player_id,
            command,
        );
        if handled && state.dirty {
            storage::export_bridge_snapshot(&state.config, &state.store);
        }
        return handled;
    }
    false
}

#[no_mangle]
pub extern "C" fn on_menu_click(
    player_id_ptr: *const u8,
    player_id_len: usize,
    menu_id_ptr: *const u8,
    menu_id_len: usize,
    slot: u8,
    click_type_ptr: *const u8,
    click_type_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let menu_id = read_str(menu_id_ptr, menu_id_len);
    let click_type = read_str(click_type_ptr, click_type_len);

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let handled = menu::handle_menu_click(
            &state.config,
            &mut state.store,
            &state.online_keys,
            &mut state.dirty,
            player_id,
            menu_id,
            slot,
            click_type,
            commands::refresh_if_online,
        );
        if handled && state.dirty {
            storage::export_bridge_snapshot(&state.config, &state.store);
        }
        return handled;
    }
    false
}

/// Cross-plugin WASM export: returns player's hierarchy weight (0..1000).
#[no_mangle]
pub extern "C" fn kronos_get_rank_weight(name_ptr: *const u8, name_len: usize) -> u32 {
    let name = read_str(name_ptr, name_len);
    let guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let key = hierarchy::normalize_key(name);
        if let Some(prof) = state.store.profiles.get(&key) {
            return hierarchy::primary_rank(&state.config, prof).weight;
        }
    }
    0
}

/// Cross-plugin WASM export: checks if player holds a permission node (with inheritance & wildcards).
#[no_mangle]
pub extern "C" fn kronos_has_permission(
    name_ptr: *const u8,
    name_len: usize,
    perm_ptr: *const u8,
    perm_len: usize,
) -> bool {
    let name = read_str(name_ptr, name_len);
    let perm = read_str(perm_ptr, perm_len);
    let guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let key = hierarchy::normalize_key(name);
        let prof = state
            .store
            .profiles
            .get(&key)
            .cloned()
            .unwrap_or_else(|| PlayerRankProfile::new(name, 0));
        return hierarchy::has_permission(&state.config, &prof, perm);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RankCategory;

    #[test]
    fn config_has_20_ranks_with_10_donate_and_9_staff_departments() {
        let cfg = storage::default_config();
        assert_eq!(cfg.ranks.len(), 20);
        assert_eq!(cfg.donate_ranks().len(), 10);
        assert_eq!(cfg.staff_ranks().len(), 9);
        assert_eq!(cfg.ranks.iter().filter(|r| r.category == RankCategory::Player).count(), 1);
        assert_eq!(cfg.find("kronos").unwrap().perks.max_homes, 50);
        assert_eq!(cfg.find("owner").unwrap().weight, 1000);
    }

    #[test]
    fn dual_track_merges_donate_and_staff_perks_and_inheritance() {
        let cfg = storage::default_config();
        let mut store = storage::load_store();
        let mut dirty = false;
        let now = 1_750_000_000_000;

        // Player buys KRONOS donate rank, then gets recruited as Trainee
        hierarchy::assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "Alex", "kronos", None, now).unwrap();
        hierarchy::assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "Alex", "trainee", None, now).unwrap();

        let prof = store.profiles.get("alex").unwrap();
        assert_eq!(hierarchy::primary_rank(&cfg, prof).id, "trainee");
        let perks = hierarchy::effective_perks(&cfg, prof);
        // Keeps 50 homes and 30 regions from KRONOS donate while working as Trainee!
        assert_eq!(perks.max_homes, 50);
        assert_eq!(perks.max_regions, 30);
        // Inherits both kronos donate perms and trainee staff perms
        assert!(hierarchy::has_permission(&cfg, prof, "kronos.chat.rgb"));
        assert!(hierarchy::has_permission(&cfg, prof, "kronos.guard.inspect"));
    }

    #[test]
    fn hr_curator_can_recruit_moderation_but_cannot_assign_tech_or_donate() {
        let cfg = storage::default_config();
        let mut store = storage::load_store();
        let mut dirty = false;
        let now = 1_750_000_000_000;

        hierarchy::assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "HrBoss", "hr_curator", None, now).unwrap();

        // Can recruit trainee & moder in MODERATION
        assert!(hierarchy::assign_rank(&cfg, &mut store, &mut dirty, Some("hrboss"), "HrBoss", false, "NewGuy", "trainee", None, now).is_ok());
        assert!(hierarchy::assign_rank(&cfg, &mut store, &mut dirty, Some("hrboss"), "HrBoss", false, "NewGuy", "moder", None, now).is_ok());
        // Cannot assign tech_admin or donate rank
        assert!(hierarchy::assign_rank(&cfg, &mut store, &mut dirty, Some("hrboss"), "HrBoss", false, "NewGuy", "tech_admin", None, now).is_err());
        assert!(hierarchy::assign_rank(&cfg, &mut store, &mut dirty, Some("hrboss"), "HrBoss", false, "NewGuy", "kronos", None, now).is_err());
    }

    #[test]
    fn temporary_donate_rank_expires_cleanly() {
        let cfg = storage::default_config();
        let mut store = storage::load_store();
        let mut dirty = false;
        let now = 1_750_000_000_000;

        hierarchy::assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "Steve", "legend", Some(60), now).unwrap();
        assert_eq!(store.profiles.get("steve").unwrap().donate_rank, "legend");

        let expired = hierarchy::expire_temp_ranks(&mut store, &mut dirty, now + 61_000);
        assert_eq!(expired.len(), 1);
        assert_eq!(store.profiles.get("steve").unwrap().donate_rank, "player");
    }
}
