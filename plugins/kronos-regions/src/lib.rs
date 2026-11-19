mod commands;
mod host;
mod menu;
mod models;
mod protection;
mod storage;
mod visualizer;

use log::info;
use models::{ActiveBorderViz, BlockPos, RegionStore, RegionsConfig, WandSelection};
use std::collections::HashMap;
use std::slice;
use std::str;
use std::sync::Mutex;

struct PluginState {
    config: RegionsConfig,
    store: RegionStore,
    selections: HashMap<String, WandSelection>,
    active_viz: HashMap<String, ActiveBorderViz>,
    last_player_region: HashMap<String, Option<String>>,
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
        selections: HashMap::new(),
        active_viz: HashMap::new(),
        last_player_region: HashMap::new(),
        dirty: false,
        last_autosave_ms: now,
        tick_counter: 0,
    });

    info!("KRONOS Regions v1.0.0 initialized (Claim Wand, 3D Borders, Granular Flags & GUIs)");
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
    info!("KRONOS Regions saved state and unloaded.");
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
        if !name.is_empty() {
            state.store.player_names.insert(player_id.to_string(), name.to_string());
            if let Some(synced_rank) = storage::resolve_rank_from_bridge(&state.config, name) {
                state.store.player_ranks.insert(player_id.to_string(), synced_rank);
            }
        }
        state.last_player_region.insert(player_id.to_string(), None);
    }
}

#[no_mangle]
pub extern "C" fn on_player_leave(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.active_viz.remove(player_id);
        state.selections.remove(player_id);
        state.last_player_region.remove(player_id);
    }
}

/// 20-TPS tick: renders 3D particle borders, tracks region entry/exit banners, and autosaves every 30s.
#[no_mangle]
pub extern "C" fn on_tick() {
    let mut guard = STATE.lock().unwrap();
    let Some(ref mut state) = *guard else {
        return;
    };
    state.tick_counter = state.tick_counter.wrapping_add(1);
    let now = host::now_ms();

    // 1. Render active 3D borders
    if !state.active_viz.is_empty() {
        let cfg = &state.config;
        state.active_viz.retain(|_, viz| {
            if now >= viz.expires_at_ms {
                return false;
            }
            visualizer::render_border_frame(viz, cfg);
            true
        });
    }

    // 2. Every 10 ticks (0.5s): detect region entry / farewell titles
    if state.tick_counter % 10 == 0 {
        let tracked_players: Vec<String> = state.last_player_region.keys().cloned().collect();
        for pid in tracked_players {
            let pos = host::get_player_block_pos(&pid);
            let current = protection::region_at(&state.store, &pos.world, pos.x, pos.y, pos.z)
                .map(|r| (r.id.clone(), r.name.clone(), r.greeting_title.clone(), r.owner_name.clone()));
            let current_id = current.as_ref().map(|(id, _, _, _)| id.clone());
            let prev_id = state.last_player_region.get(&pid).cloned().flatten();

            if current_id != prev_id {
                state.last_player_region.insert(pid.clone(), current_id);
                if let Some((_, name, greeting, owner)) = current {
                    if let Some(custom) = greeting {
                        host::send_title(&pid, &format!("§6✦ {}", name), &format!("§f{}", custom), 5, 35, 10);
                    } else {
                        host::send_action_bar(&pid, &format!("§6✦ §fВы вошли в приват §e{} §8(§7{}§8)", name, owner));
                    }
                } else if prev_id.is_some() {
                    host::send_action_bar(&pid, "§7Вы вышли на свободную территорию");
                }
            }
        }
    }

    // 3. Debounced atomic autosave every 30s
    if state.tick_counter % 20 == 0 && state.dirty {
        let interval_ms = state.config.autosave_interval_seconds.max(5) * 1000;
        if now.saturating_sub(state.last_autosave_ms) >= interval_ms {
            storage::save_store_atomic(&state.store);
            state.dirty = false;
            state.last_autosave_ms = now;
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
            &mut state.selections,
            &mut state.active_viz,
            &mut state.dirty,
            player_id,
            command,
        );
    }
    false
}

#[no_mangle]
pub extern "C" fn on_wand_click(
    player_id_ptr: *const u8,
    player_id_len: usize,
    click_type_ptr: *const u8,
    click_type_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let click_type = read_str(click_type_ptr, click_type_len);
    let world = read_str(world_ptr, world_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        return commands::handle_wand_click(
            &state.store,
            &state.config,
            &mut state.selections,
            &mut state.active_viz,
            player_id,
            click_type,
            BlockPos { world: world.to_string(), x, y, z },
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
        let mut ctx = menu::RegionMenuCtx {
            store: &mut state.store,
            config: &state.config,
            selections: &mut state.selections,
            active_viz: &mut state.active_viz,
            dirty: &mut state.dirty,
        };
        return menu::handle_menu_click(
            &mut ctx,
            player_id,
            menu_id,
            slot,
            click_type,
            commands::try_claim_selection,
            commands::try_resize_region,
        );
    }
    false
}

#[no_mangle]
pub extern "C" fn on_block_break(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_modify_block(&s.store, &s.config, player_id, world, x, y, z))
        .unwrap_or(true)
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
) -> bool {
    on_block_break(player_id_ptr, player_id_len, world_ptr, world_len, x, y, z)
}

#[no_mangle]
pub extern "C" fn on_block_interact(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    block_id_ptr: *const u8,
    block_id_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let block_id = read_str(block_id_ptr, block_id_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_interact_block(&s.store, &s.config, player_id, world, x, y, z, block_id))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_explosion(world_ptr: *const u8, world_len: usize, x: i32, y: i32, z: i32) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_explode(&s.store, world, x, y, z))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_pvp_damage(
    attacker_ptr: *const u8,
    attacker_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    let attacker = read_str(attacker_ptr, attacker_len);
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_pvp_damage(&s.store, attacker, world, x, y, z))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_fall_damage(world_ptr: *const u8, world_len: usize, x: i32, y: i32, z: i32) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_fall_damage(&s.store, world, x, y, z))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_mob_spawn(
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    hostile: bool,
) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_mob_spawn(&s.store, world, x, y, z, hostile))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_fire_spread(world_ptr: *const u8, world_len: usize, x: i32, y: i32, z: i32) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_fire_spread(&s.store, world, x, y, z))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ExpandDirection, Region, RegionBounds, RegionFlags};

    #[test]
    fn aabb_overlap_and_directional_expansion_work() {
        let a = RegionBounds {
            world: "world".into(),
            min_x: 0, min_y: 60, min_z: 0,
            max_x: 9, max_y: 69, max_z: 9,
        };
        assert_eq!(a.volume(), 1000);

        let expanded = a.resized(ExpandDirection::East, 5, -64, 319);
        assert_eq!(expanded.max_x, 14);
        assert_eq!(expanded.volume(), 1500);

        let vert = a.resized(ExpandDirection::Vert, 0, -64, 319);
        assert_eq!(vert.min_y, -64);
        assert_eq!(vert.max_y, 319);

        let far = RegionBounds {
            world: "world".into(),
            min_x: 20, min_y: 60, min_z: 20,
            max_x: 30, max_y: 70, max_z: 30,
        };
        assert!(!a.intersects(&far));
        assert!(a.intersects(&expanded));
    }

    #[test]
    fn granular_flags_control_chests_doors_buttons_and_damage_independently() {
        let cfg = storage::default_config();
        let mut store = RegionStore::default();
        let mut flags = RegionFlags::default();
        flags.use_doors = true;
        flags.open_chests = false;
        flags.use_buttons_levers = false;
        flags.pvp = false;
        flags.fall_damage = false;
        flags.explosions = false;

        store.regions.insert(
            "rg_1".into(),
            Region {
                id: "rg_1".into(),
                name: "castle".into(),
                owner_id: "steve".into(),
                owner_name: "Steve".into(),
                bounds: RegionBounds {
                    world: "world".into(),
                    min_x: 0, min_y: 0, min_z: 0,
                    max_x: 20, max_y: 100, max_z: 20,
                },
                flags,
                members: HashMap::new(),
                border_theme: "emerald_wire".into(),
                greeting_title: None,
                created_at_ms: 0,
            },
        );

        assert!(protection::can_interact_block(&store, &cfg, "stranger", "world", 5, 65, 5, "minecraft:oak_door"));
        assert!(!protection::can_interact_block(&store, &cfg, "stranger", "world", 5, 65, 5, "minecraft:chest"));
        assert!(!protection::can_interact_block(&store, &cfg, "stranger", "world", 5, 65, 5, "minecraft:stone_button"));
        assert!(protection::can_interact_block(&store, &cfg, "steve", "world", 5, 65, 5, "minecraft:chest"));
        assert!(!protection::can_pvp_damage(&store, "stranger", "world", 5, 65, 5));
        assert!(!protection::can_fall_damage(&store, "world", 5, 65, 5));
        assert!(!protection::can_explode(&store, "world", 5, 65, 5));
    }

    #[test]
    fn rank_monetization_tiers_gate_premium_flags_and_themes() {
        let cfg = storage::default_config();
        assert!(!cfg.rank_for("player").can_toggle_pvp);
        assert!(cfg.rank_for("vip").can_toggle_pvp);
        assert!(!cfg.rank_for("vip").can_toggle_fall_damage);
        assert!(cfg.rank_for("premium").can_toggle_fall_damage);
        assert!(cfg.rank_for("legend").can_toggle_entry);
        assert!(!visualizer::can_use_border_theme(0, "golden_beam"));
        assert!(visualizer::can_use_border_theme(1, "golden_beam"));
        assert!(visualizer::can_use_border_theme(3, "dragon_aurora"));
    }
}
