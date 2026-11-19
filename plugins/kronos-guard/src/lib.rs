mod chat_filter;
mod commands;
mod host;
mod inspector;
mod menu;
mod models;
mod punishments;
mod storage;

use log::info;
use models::{
    parse_duration_seconds, BlockAction, BlockLogEntry, GuardConfig, GuardStore, PunishmentKind,
    RuntimeSession,
};
use std::collections::VecDeque;
use std::slice;
use std::str;
use std::sync::Mutex;

struct PluginState {
    config: GuardConfig,
    store: GuardStore,
    block_log: VecDeque<BlockLogEntry>,
    session: RuntimeSession,
    dirty_store: bool,
    dirty_log: bool,
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
    let block_log = storage::load_block_log();
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    *guard = Some(PluginState {
        config,
        store,
        block_log,
        session: RuntimeSession::default(),
        dirty_store: false,
        dirty_log: false,
        last_autosave_ms: now,
        tick_counter: 0,
    });
    info!("KRONOS Guard & Moderation v1.0.0 initialized (Punishments, Vanish, ChatGuard, BlockLog)");
    0
}

#[no_mangle]
pub extern "C" fn cleanup() {
    let mut guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        if state.dirty_store {
            storage::save_store_atomic(&state.store);
        }
        if state.dirty_log {
            storage::save_block_log_atomic(&state.block_log);
        }
    }
    *guard = None;
    info!("KRONOS Guard saved state and unloaded.");
}

/// Login gate: blocks banned / IP-banned players before they enter the world.
#[no_mangle]
pub extern "C" fn on_player_pre_login(
    player_id_ptr: *const u8,
    player_id_len: usize,
    name_ptr: *const u8,
    name_len: usize,
    ip_ptr: *const u8,
    ip_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let name = read_str(name_ptr, name_len);
    let ip = read_str(ip_ptr, ip_len);
    let now = host::now_ms();

    let guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let ip_opt = if ip.is_empty() { None } else { Some(ip) };
        if let Some(ban) = punishments::active_ban(&state.store, name, ip_opt, now) {
            let screen = punishments::format_ban_screen(&state.config, ban, now);
            host::kick_player(player_id, &screen);
            return false;
        }
    }
    true
}

#[no_mangle]
pub extern "C" fn on_player_join(
    player_id_ptr: *const u8,
    player_id_len: usize,
    name_ptr: *const u8,
    name_len: usize,
    ip_ptr: *const u8,
    ip_len: usize,
) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let name = read_str(name_ptr, name_len);
    let ip = read_str(ip_ptr, ip_len);

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let key = punishments::normalize_key(name);
        state.session.online_ids.insert(key.clone(), player_id.to_string());
        if !name.is_empty() {
            state.store.known_names.insert(player_id.to_string(), name.to_string());
        }
        if !ip.is_empty() {
            state.store.known_ips.insert(key.clone(), ip.to_string());
        }
        let rank = storage::resolve_rank_from_bridge(name)
            .unwrap_or_else(|| host::get_player_rank(player_id));
        state.store.known_ranks.insert(key, rank);
    }
}

/// Handles disconnect cleanup and auto-bans players who log out while frozen during a check.
#[no_mangle]
pub extern "C" fn on_player_leave(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let name = state
            .store
            .known_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.to_string());
        let key = punishments::normalize_key(&name);

        state.session.online_ids.remove(&key);
        state.session.inspectors.remove(player_id);
        state.session.vanished.remove(player_id);

        if state.session.frozen.remove(&key).is_some() {
            let secs = parse_duration_seconds(&state.config.freeze_logout_ban_duration)
                .unwrap_or(Some(604_800));
            let _ = punishments::issue_punishment(
                &mut state.store,
                &state.config,
                &state.session,
                &mut state.dirty_store,
                "CONSOLE",
                "KRONOS AntiEvasion",
                "owner",
                PunishmentKind::TempBan,
                &name,
                secs,
                "Выход с сервера во время заморозки (проверки)",
                now,
            );
        }
    }
}

/// 20-TPS tick: vanish ActionBar HUD, retention log pruning, and debounced atomic autosave.
#[no_mangle]
pub extern "C" fn on_tick() {
    let mut guard = STATE.lock().unwrap();
    let Some(ref mut state) = *guard else {
        return;
    };
    state.tick_counter = state.tick_counter.wrapping_add(1);
    let now = host::now_ms();

    // Every 20 ticks (1s): show Vanish HUD bar
    if state.tick_counter % 20 == 0 && state.config.vanish.show_action_bar {
        for pid in &state.session.vanished {
            host::send_action_bar(pid, "§b👻 ВЫ В ВАНИШЕ §8| §7Скрыт от игроков");
        }
    }

    // Every 600 ticks (30s): prune expired block log entries
    if state.tick_counter % 600 == 0 {
        inspector::prune_old_entries(&mut state.block_log, &state.config, &mut state.dirty_log, now);
    }

    // Debounced atomic autosave
    if state.tick_counter % 20 == 0 && (state.dirty_store || state.dirty_log) {
        let interval_ms = state.config.autosave_interval_seconds.max(5) * 1000;
        if now.saturating_sub(state.last_autosave_ms) >= interval_ms {
            if state.dirty_store {
                storage::save_store_atomic(&state.store);
                state.dirty_store = false;
            }
            if state.dirty_log {
                storage::save_block_log_atomic(&state.block_log);
                state.dirty_log = false;
            }
            state.last_autosave_ms = now;
        }
    }
}

/// Chat guard hook: returns true to allow message, false to cancel it.
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

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let name = state
            .store
            .known_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.to_string());
        let decision = chat_filter::evaluate_chat(
            &mut state.store,
            &state.config,
            &mut state.session,
            &mut state.dirty_store,
            player_id,
            &name,
            message,
            now,
        );
        return decision == chat_filter::ChatDecision::Allow;
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
        return commands::handle_command(
            &mut state.store,
            &mut state.block_log,
            &state.config,
            &mut state.session,
            &mut state.dirty_store,
            &mut state.dirty_log,
            player_id,
            command,
        );
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
        return menu::handle_menu_click(
            &mut state.store,
            &state.block_log,
            &state.config,
            &mut state.session,
            &mut state.dirty_store,
            player_id,
            menu_id,
            slot,
            click_type,
            commands::toggle_vanish,
            commands::toggle_inspector,
            commands::toggle_freeze,
        );
    }
    false
}

/// Logs block break or intercepts click when staff has Inspector mode (`/co i`) enabled.
/// Returns true to allow normal break, false if intercepted by inspector mode.
#[no_mangle]
pub extern "C" fn on_block_break(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    block_ptr: *const u8,
    block_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let block = read_str(block_ptr, block_len);
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if state.session.inspectors.contains(player_id) {
            inspector::inspect_block_for_staff(
                &state.block_log,
                &state.config,
                &mut state.session,
                player_id,
                world,
                x,
                y,
                z,
                now,
            );
            return false;
        }

        let actor = state
            .store
            .known_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.to_string());
        inspector::record_event(
            &mut state.block_log,
            &state.config,
            &mut state.dirty_log,
            world,
            x,
            y,
            z,
            &actor,
            BlockAction::Break,
            block,
            None,
            None,
            now,
        );
    }
    true
}

#[no_mangle]
pub extern "C" fn on_block_place(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    block_ptr: *const u8,
    block_len: usize,
    prev_block_ptr: *const u8,
    prev_block_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let block = read_str(block_ptr, block_len);
    let prev_block = read_str(prev_block_ptr, prev_block_len);
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if state.session.inspectors.contains(player_id) {
            inspector::inspect_block_for_staff(
                &state.block_log,
                &state.config,
                &mut state.session,
                player_id,
                world,
                x,
                y,
                z,
                now,
            );
            return false;
        }

        let actor = state
            .store
            .known_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.to_string());
        let action = if block.contains("lava") {
            BlockAction::LavaPlace
        } else if block.contains("water") {
            BlockAction::WaterPlace
        } else {
            BlockAction::Place
        };
        let prev = if prev_block.is_empty() {
            Some("minecraft:air".into())
        } else {
            Some(prev_block.to_string())
        };
        inspector::record_event(
            &mut state.block_log,
            &state.config,
            &mut state.dirty_log,
            world,
            x,
            y,
            z,
            &actor,
            action,
            block,
            None,
            prev,
            now,
        );
    }
    true
}

/// Logs container access (chest, barrel, shulker) or inspects chest history if Inspector mode is active.
#[no_mangle]
pub extern "C" fn on_container_open(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    container_ptr: *const u8,
    container_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let container = read_str(container_ptr, container_len);
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if state.session.inspectors.contains(player_id) {
            inspector::inspect_block_for_staff(
                &state.block_log,
                &state.config,
                &mut state.session,
                player_id,
                world,
                x,
                y,
                z,
                now,
            );
            return false;
        }

        // Vanished staff open containers silently without polluting audit log
        if state.session.vanished.contains(player_id) && state.config.vanish.silent_containers {
            return true;
        }

        let actor = state
            .store
            .known_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.to_string());
        inspector::record_event(
            &mut state.block_log,
            &state.config,
            &mut state.dirty_log,
            world,
            x,
            y,
            z,
            &actor,
            BlockAction::ContainerOpen,
            container,
            None,
            None,
            now,
        );
    }
    true
}

/// Logs individual item withdrawals or deposits in chests/barrels/shulkers.
#[no_mangle]
pub extern "C" fn on_container_transfer(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    container_ptr: *const u8,
    container_len: usize,
    item_ptr: *const u8,
    item_len: usize,
    is_take: bool,
) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let container = read_str(container_ptr, container_len);
    let item = read_str(item_ptr, item_len);
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let actor = state
            .store
            .known_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.to_string());
        inspector::record_event(
            &mut state.block_log,
            &state.config,
            &mut state.dirty_log,
            world,
            x,
            y,
            z,
            &actor,
            if is_take {
                BlockAction::ContainerTake
            } else {
                BlockAction::ContainerPut
            },
            container,
            Some(item.to_string()),
            None,
            now,
        );
    }
}

#[no_mangle]
pub extern "C" fn on_explosion(
    source_ptr: *const u8,
    source_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    destroyed_block_ptr: *const u8,
    destroyed_block_len: usize,
) {
    let source = read_str(source_ptr, source_len);
    let world = read_str(world_ptr, world_len);
    let block = read_str(destroyed_block_ptr, destroyed_block_len);
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let actor = if source.is_empty() { "#tnt" } else { source };
        inspector::record_event(
            &mut state.block_log,
            &state.config,
            &mut state.dirty_log,
            world,
            x,
            y,
            z,
            actor,
            BlockAction::Explosion,
            block,
            None,
            None,
            now,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{format_duration_ru, parse_duration_seconds};

    #[test]
    fn duration_parser_and_formatter_work() {
        assert_eq!(parse_duration_seconds("30s"), Ok(Some(30)));
        assert_eq!(parse_duration_seconds("15m"), Ok(Some(900)));
        assert_eq!(parse_duration_seconds("2h"), Ok(Some(7200)));
        assert_eq!(parse_duration_seconds("7d"), Ok(Some(604_800)));
        assert_eq!(parse_duration_seconds("perm"), Ok(None));
        assert!(parse_duration_seconds("0m").is_err());
        assert_eq!(format_duration_ru(90_061), "1д 1ч 1м");
    }

    #[test]
    fn hierarchy_blocks_lower_staff_and_warn_ladder_auto_escalates() {
        let cfg = storage::default_config();
        assert_eq!(cfg.staff_ranks.len(), 20);
        assert!(cfg.perms_for("trainee").weight < cfg.perms_for("helper").weight);
        assert!(cfg.perms_for("hr_curator").weight < cfg.perms_for("survival_admin").weight);
        assert!(cfg.perms_for("survival_admin").weight < cfg.perms_for("tech_admin").weight);
        assert!(cfg.perms_for("tech_admin").weight < cfg.perms_for("curator").weight);
        assert!(cfg.perms_for("curator").weight < cfg.perms_for("owner").weight);

        let mut store = storage::load_store();
        store.known_ranks.insert("boss".into(), "tech_admin".into());
        let session = RuntimeSession::default();
        let mut dirty = false;
        let now = 1_750_000_000_000;

        // Moder cannot ban tech_admin
        let blocked = punishments::issue_punishment(
            &mut store,
            &cfg,
            &session,
            &mut dirty,
            "moder",
            "ModerOne",
            "moder",
            PunishmentKind::TempBan,
            "boss",
            Some(3600),
            "test",
            now,
        );
        assert!(blocked.is_err());

        // 2 warns -> automatic 30m mute via warn_escalation
        assert!(punishments::issue_punishment(
            &mut store,
            &cfg,
            &session,
            &mut dirty,
            "moder",
            "ModerOne",
            "moder",
            PunishmentKind::Warn,
            "griefer",
            None,
            "warn 1",
            now,
        )
        .is_ok());
        assert!(punishments::active_mute(&store, "griefer", now).is_none());

        assert!(punishments::issue_punishment(
            &mut store,
            &cfg,
            &session,
            &mut dirty,
            "moder",
            "ModerOne",
            "moder",
            PunishmentKind::Warn,
            "griefer",
            None,
            "warn 2",
            now + 1000,
        )
        .is_ok());
        assert!(punishments::active_mute(&store, "griefer", now + 2000).is_some());
    }

    #[test]
    fn anti_advertising_catches_obfuscated_domains_and_respects_whitelist() {
        let wl = vec!["kronosmc.ru".to_string(), "discord.gg/kronos".to_string()];
        assert!(chat_filter::contains_advertisement("заходите на 185.22.14.9:25565", &wl));
        assert!(chat_filter::contains_advertisement("топ сервер play . rivalmc . ru", &wl));
        assert!(chat_filter::contains_advertisement("мой канал t.me/cheatshop", &wl));
        assert!(!chat_filter::contains_advertisement("залетайте на kronosmc.ru и discord.gg/kronos!", &wl));
        assert!(!chat_filter::contains_advertisement("продам 32 алмаза на спавне", &wl));
    }

    #[test]
    fn compact_block_and_chest_log_lookup_and_rollback_work() {
        let cfg = storage::default_config();
        let mut store = storage::load_store();
        let mut log = VecDeque::new();
        let mut session = RuntimeSession::default();
        let mut dirty = false;
        let now = 1_750_000_000_000;

        inspector::record_event(
            &mut log,
            &cfg,
            &mut dirty,
            "world",
            10,
            64,
            10,
            "Griefer",
            BlockAction::Break,
            "minecraft:diamond_block",
            None,
            None,
            now,
        );
        inspector::record_event(
            &mut log,
            &cfg,
            &mut dirty,
            "world",
            10,
            64,
            10,
            "Griefer",
            BlockAction::ContainerTake,
            "minecraft:chest",
            Some("minecraft:netherite_ingot x4".into()),
            None,
            now + 5_000,
        );

        let hits = inspector::lookup_at(&log, "world", 10, 64, 10, 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].action, BlockAction::ContainerTake);
        assert_eq!(hits[1].action, BlockAction::Break);

        let rolled = inspector::rollback_grief(
            &mut log,
            &mut store,
            &cfg,
            &mut session,
            &mut dirty,
            "admin",
            "Admin",
            "world",
            10,
            64,
            10,
            5,
            (now / 1000) - 60,
            Some("Griefer"),
            now + 10_000,
        )
        .unwrap();
        assert_eq!(rolled, 1);
        assert_eq!(inspector::undo_last_rollback(&mut session, "admin").unwrap(), 1);
    }
}
