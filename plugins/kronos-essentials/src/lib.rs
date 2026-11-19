mod commands;
mod effects;
mod host;
mod menu;
mod models;
mod storage;

use log::info;
use models::{
    ActiveMeteorFall, CombatTag, EssentialsConfig, EssentialsStore, PendingTeleport, PlayerRecord,
    TpaRequest,
};
use std::collections::{HashMap, HashSet};
use std::slice;
use std::str;
use std::sync::Mutex;

struct PluginState {
    config: EssentialsConfig,
    store: EssentialsStore,
    online_players: HashSet<String>,
    pending_teleports: HashMap<String, PendingTeleport>,
    active_meteors: HashMap<String, ActiveMeteorFall>,
    tpa_requests: HashMap<String, TpaRequest>,
    combat_tags: HashMap<String, CombatTag>,
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

    let mut guard = STATE.lock().unwrap();
    *guard = Some(PluginState {
        config,
        store,
        online_players: HashSet::new(),
        pending_teleports: HashMap::new(),
        active_meteors: HashMap::new(),
        tpa_requests: HashMap::new(),
        combat_tags: HashMap::new(),
        dirty: false,
        last_autosave_ms: now,
        tick_counter: 0,
    });

    info!("KRONOS Essentials v2.0.0 initialized (Homes, RTP, Anti-Combat, 5 GUIs, Atomic Save)");
    0
}

#[no_mangle]
pub extern "C" fn cleanup() {
    let mut guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        if state.dirty {
            storage::save_store_atomic(&state.store);
        }
    }
    *guard = None;
    info!("KRONOS Essentials v2.0.0 flushed state and unloaded.");
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

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.online_players.insert(player_id.to_string());
        let entry = state
            .store
            .players
            .entry(player_id.to_string())
            .or_insert_with(|| PlayerRecord::new(player_id, name));
        if !name.is_empty() {
            entry.username = name.to_string();
            if let Some((primary, donate)) = storage::resolve_ranks_from_bridge(name) {
                entry.rank = primary;
                entry.donate_rank = Some(donate);
            }
        }

        // Teleport first-time players to spawn if configured
        let is_first_join = !entry.first_join_completed;
        if is_first_join {
            entry.first_join_completed = true;
            state.dirty = true;
        }

        let rank = state.config.effective_rank_for(entry);
        let limit = if rank.unlimited_homes {
            "∞".to_string()
        } else {
            rank.max_homes.to_string()
        };
        let home_count = entry.homes.len();
        let display_name = entry.username.clone();

        if is_first_join && state.config.teleport_new_players_to_spawn {
            if let Some(spawn) = state.store.spawn.clone() {
                host::teleport_player(player_id, &spawn);
                host::send_title(
                    player_id,
                    "§6✦ Добро пожаловать на KRONOS",
                    "§7Откройте меню телепортов: §e/homes",
                    10,
                    60,
                    20,
                );
            }
        }

        host::send_chat(
            player_id,
            &format!(
                "§6✦ §fПривет, {}§f{}! Метки дома: §e{}/{} §7(/homes • /rtp)",
                rank.display_prefix, display_name, home_count, limit
            ),
        );
    }
}

#[no_mangle]
pub extern "C" fn on_player_leave(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.online_players.remove(player_id);
        state.pending_teleports.remove(player_id);
        state.active_meteors.remove(player_id);
        state.tpa_requests.remove(player_id);
        if state.dirty {
            storage::save_store_atomic(&state.store);
            state.dirty = false;
            state.last_autosave_ms = host::now_ms();
        }
    }
}

/// Called every server tick (20 TPS):
/// - Animates teleport spirals & meteor trails
/// - Checks movement cancellation & completes teleports
/// - Displays PvP Combat ActionBar timers
/// - Cleans expired TPA/combat entries & runs debounced atomic autosave every 30s
#[no_mangle]
pub extern "C" fn on_tick() {
    let mut guard = STATE.lock().unwrap();
    let Some(ref mut state) = *guard else {
        return;
    };

    state.tick_counter = state.tick_counter.wrapping_add(1);
    let now = host::now_ms();

    // 1. Warmup teleports
    if !state.pending_teleports.is_empty() {
        let max_move_sq = state.config.cancel_on_move_blocks * state.config.cancel_on_move_blocks;
        let mut completed: Vec<String> = Vec::new();
        let mut cancelled: Vec<(String, String)> = Vec::new();

        for (pid, pending) in state.pending_teleports.iter_mut() {
            let current_loc = host::get_player_location(pid);
            if current_loc.distance_squared(&pending.origin) > max_move_sq {
                cancelled.push((pid.clone(), "Вы сдвинулись с места".to_string()));
                continue;
            }
            if now.saturating_sub(pending.started_at_ms) >= pending.duration_ms {
                completed.push(pid.clone());
            } else {
                effects::render_warmup_frame(pending, now);
            }
        }

        for (pid, reason) in cancelled {
            if let Some(pending) = state.pending_teleports.remove(&pid) {
                effects::play_cancel_effect(&pid, &pending.origin, &reason);
            }
        }

        for pid in completed {
            if let Some(pending) = state.pending_teleports.remove(&pid) {
                if let Some(player) = state.store.players.get_mut(&pid) {
                    player.last_location = Some(pending.origin.clone());
                    player.last_teleport_ms = now;
                    state.dirty = true;
                }
                if pending.is_rtp_meteor {
                    let meteor = effects::play_rtp_meteor_launch(
                        &pid,
                        &pending.origin,
                        &pending.destination,
                        &state.config,
                        now,
                    );
                    state.active_meteors.insert(pid, meteor);
                } else {
                    effects::play_arrival_burst(
                        &pid,
                        &pending.target_label,
                        &pending.origin,
                        &pending.destination,
                        &pending.theme,
                        &state.config,
                    );
                }
            }
        }
    }

    // 2. Active RTP meteor drop particle trails
    if !state.active_meteors.is_empty() {
        state.active_meteors.retain(|_, fall| {
            if now >= fall.expires_at_ms {
                return false;
            }
            effects::render_meteor_trail_frame(fall);
            true
        });
    }

    // 3. Every 1 second (20 ticks): combat ActionBar countdown + expired TPA/combat cleanup
    if state.tick_counter % 20 == 0 {
        state.tpa_requests.retain(|_, req| req.expires_at_ms > now);
        state.combat_tags.retain(|pid, tag| {
            if tag.expires_at_ms <= now {
                host::send_action_bar(pid, "§a✔ Вы вышли из боя — телепорты снова доступны");
                return false;
            }
            let left = (tag.expires_at_ms - now + 999) / 1000;
            if !state.pending_teleports.contains_key(pid) {
                host::send_action_bar(
                    pid,
                    &format!("§c⚔ Режим боя ({}) §8• §e{}с §7(телепорт заблокирован)", tag.opponent, left),
                );
            }
            true
        });

        // Every 5 seconds (100 ticks): live-sync dual-track ranks from kronos-ranks bridge
        if state.tick_counter % 100 == 0 && !state.online_players.is_empty() {
            let online: Vec<String> = state.online_players.iter().cloned().collect();
            storage::refresh_online_ranks_from_bridge(&mut state.store, online.into_iter());
        }

        // Debounced atomic autosave
        let interval_ms = state.config.autosave_interval_seconds.max(5) * 1000;
        if state.dirty && now.saturating_sub(state.last_autosave_ms) >= interval_ms {
            storage::save_store_atomic(&state.store);
            state.dirty = false;
            state.last_autosave_ms = now;
        }
    }
}

/// Cancels warmup teleports on damage and tags both players in PvP combat if attacker_id is provided.
#[no_mangle]
pub extern "C" fn on_player_damage(
    player_id_ptr: *const u8,
    player_id_len: usize,
    attacker_id_ptr: *const u8,
    attacker_id_len: usize,
) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let attacker_id = read_str(attacker_id_ptr, attacker_id_len);

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if state.config.cancel_on_damage {
            if let Some(pending) = state.pending_teleports.remove(player_id) {
                effects::play_cancel_effect(player_id, &pending.origin, "Получен урон");
            }
        }

        if !attacker_id.is_empty() && attacker_id != player_id && state.config.combat_tag_seconds > 0 {
            let expires = host::now_ms() + state.config.combat_tag_seconds * 1000;
            state.combat_tags.insert(
                player_id.to_string(),
                CombatTag {
                    opponent: attacker_id.to_string(),
                    expires_at_ms: expires,
                },
            );
            state.combat_tags.insert(
                attacker_id.to_string(),
                CombatTag {
                    opponent: player_id.to_string(),
                    expires_at_ms: expires,
                },
            );
        }
    }
}

/// Saves death location for VIP+ /back and clears combat tag.
#[no_mangle]
pub extern "C" fn on_player_death(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let death_loc = host::get_player_location(player_id);
        state.combat_tags.remove(player_id);
        state.pending_teleports.remove(player_id);

        let entry = state
            .store
            .players
            .entry(player_id.to_string())
            .or_insert_with(|| PlayerRecord::new(player_id, player_id));
        entry.last_death_location = Some(death_loc);
        state.dirty = true;

        let rank = state.config.effective_rank_for(entry);
        if rank.can_back_on_death {
            host::send_chat(player_id, "§e☠ Точка смерти сохранена! Вернитесь командой §a/back");
        }
    }
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
            &state.config,
            &mut state.pending_teleports,
            &mut state.tpa_requests,
            &state.combat_tags,
            &mut state.dirty,
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
        let mut ctx = menu::MenuDispatchContext {
            store: &mut state.store,
            config: &state.config,
            pending: &mut state.pending_teleports,
            tpa_requests: &mut state.tpa_requests,
            combat_tags: &state.combat_tags,
            dirty: &mut state.dirty,
        };
        return menu::handle_menu_click(
            &mut ctx,
            player_id,
            menu_id,
            slot,
            click_type,
            commands::schedule_teleport,
            commands::trigger_rtp,
        );
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_home_limits_scale_properly() {
        let cfg = storage::default_config();
        assert_eq!(cfg.ranks.len(), 20);
        assert_eq!(cfg.rank_for("player").max_homes, 3);
        assert_eq!(cfg.rank_for("vip").max_homes, 5);
        assert_eq!(cfg.rank_for("premium").max_homes, 7);
        assert_eq!(cfg.rank_for("deluxe").max_homes, 9);
        assert_eq!(cfg.rank_for("legend").max_homes, 16);
        assert_eq!(cfg.rank_for("kronos").max_homes, 50);
        assert!(cfg.rank_for("survival_admin").unlimited_homes);
        assert!(cfg.rank_for("tech_admin").unlimited_homes);
        assert!(cfg.rank_for("owner").unlimited_homes);
    }

    #[test]
    fn effects_are_gated_by_rank_tier() {
        assert!(effects::can_use_theme(0, "amethyst_spiral"));
        assert!(!effects::can_use_theme(0, "golden_vortex"));
        assert!(effects::can_use_theme(1, "golden_vortex"));
        assert!(!effects::can_use_theme(2, "dragon_nova"));
        assert!(effects::can_use_theme(3, "dragon_nova"));
        assert!(effects::can_use_theme(4, "kronos_rift"));
    }

    #[test]
    fn rtp_coordinates_stay_within_configured_radius() {
        let mut seed = 123456789u64;
        for _ in 0..50 {
            let (x, z) = commands::sample_rtp_xz(&mut seed, 250, 4500);
            let dist = (x * x + z * z).sqrt();
            assert!(dist >= 248.0 && dist <= 4502.0);
        }
    }

    #[test]
    fn combat_tag_blocks_until_expiry() {
        let mut tags = HashMap::new();
        tags.insert(
            "steve".to_string(),
            CombatTag {
                opponent: "alex".to_string(),
                expires_at_ms: 10_000,
            },
        );
        assert_eq!(commands::remaining_combat_seconds(&tags, "steve", 7_500), 3);
        assert_eq!(commands::remaining_combat_seconds(&tags, "steve", 10_500), 0);
    }

    #[test]
    fn dual_track_staff_preserves_kronos_homes_and_per_rank_rtp_cooldowns() {
        let cfg = storage::default_config();
        let mut rec = PlayerRecord::new("uuid-1", "Alex");
        rec.rank = "trainee".into();
        rec.donate_rank = Some("kronos".into());

        let eff = cfg.effective_rank_for(&rec);
        assert_eq!(eff.max_homes, 50);
        assert_eq!(eff.warmup_seconds, 0);
        assert_eq!(eff.rtp_cooldown_seconds, Some(0));
        assert_eq!(eff.tier, 4);
        assert_eq!(cfg.rank_for("legend").rtp_cooldown_seconds, Some(45));
        assert_eq!(cfg.next_donate_after(&cfg.rank_for("premium")).unwrap().id, "deluxe");
        assert!(cfg.next_donate_after(&cfg.rank_for("kronos")).is_none());
    }
}
