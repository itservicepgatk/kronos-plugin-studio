pub mod commands;
pub mod effects;
pub mod menu;
pub mod models;
pub mod storage;

use pumpkin_plugin_api::{
    events::{
        BedrockFormResponseEvent, EntityDamageByEntityEvent, EntityDamageEvent, EventHandler,
        EventPriority, PlayerDeathEvent, PlayerJoinEvent, PlayerLeaveEvent,
    },
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    wit::pumpkin::plugin::event::{
        BedrockFormResponseEventData, EntityDamageByEntityEventData, EntityDamageEventData,
        PlayerDeathEventData, PlayerJoinEventData, PlayerLeaveEventData,
    },
    Context, Player, Plugin, PluginMetadata, Server,
};

use crate::commands::{execute_bedrock_action, PERM_ADMIN, PERM_PLAYER};
use crate::effects::{capture_player_location, execute_teleport, legacy, play_error_sound};
use crate::models::{normalize_key, LocationPoint};
use crate::storage::{
    init_storage, now_ms, save_current_store, set_death_location, with_state,
};

fn apply_combat_tag_to_player(player: &Player) {
    let p_name = player.get_name();
    let now = now_ms();
    let (secs, should_notify) = with_state(|st| st.tag_combat(&p_name, now));

    let msg = format!("§c⚔ Вы в бою! Телепортация заблокирована на {}с", secs);
    player.show_actionbar(legacy(&msg));
    if should_notify {
        player.send_system_message(legacy(&msg), false);
    }
}

fn find_player_by_entity_id(server: &Server, entity_id: i32) -> Option<Player> {
    server
        .get_all_players()
        .into_iter()
        .find(|p| p.as_entity().get_id() as i32 == entity_id)
}

// =============================================================================
// Event Listeners (Anti-Combat PvP Timer, Death /back, Join Spawn, Bedrock Forms)
// =============================================================================

struct PvPCombatHandler;
impl EventHandler<EntityDamageByEntityEvent> for PvPCombatHandler {
    fn handle(
        &self,
        server: Server,
        event: EntityDamageByEntityEventData,
    ) -> EntityDamageByEntityEventData {
        if !event.cancelled && event.damage > 0.0 {
            if let Some(victim) = find_player_by_entity_id(&server, event.entity_id) {
                apply_combat_tag_to_player(&victim);
            }
            if event.damager_id != event.entity_id {
                if let Some(attacker) = find_player_by_entity_id(&server, event.damager_id) {
                    apply_combat_tag_to_player(&attacker);
                }
            }
        }
        event
    }
}

struct DamageCombatHandler;
impl EventHandler<EntityDamageEvent> for DamageCombatHandler {
    fn handle(&self, server: Server, event: EntityDamageEventData) -> EntityDamageEventData {
        if !event.cancelled && event.damage > 0.0 {
            if let Some(victim) = find_player_by_entity_id(&server, event.entity_id) {
                apply_combat_tag_to_player(&victim);
            }
        }
        event
    }
}

struct DeathHandler;
impl EventHandler<PlayerDeathEvent> for DeathHandler {
    fn handle(&self, _server: Server, event: PlayerDeathEventData) -> PlayerDeathEventData {
        if !event.cancelled {
            let p_name = event.player.get_name();
            let death_loc = capture_player_location(&event.player);
            with_state(|st| {
                st.clear_combat(&p_name);
                set_death_location(&mut st.store, &p_name, death_loc);
            });
            save_current_store();
            event.player.send_system_message(
                legacy("§6§l✦ KRONOS §8» §7Точка смерти сохранена. Используйте §e/back §7для возврата."),
                false,
            );
        }
        event
    }
}

struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let p_name = event.player.get_name();
        let (should_tp_spawn, spawn_loc, cfg) = with_state(|st| {
            let entry = st.store.players.entry(normalize_key(&p_name)).or_default();
            let first_join = !entry.visited_before;
            entry.visited_before = true;
            let tp_spawn = first_join && st.config.teleport_new_players_to_spawn;
            let spawn = st
                .store
                .spawn
                .clone()
                .unwrap_or_else(LocationPoint::default_spawn);
            (tp_spawn, spawn, st.config.clone())
        });

        if should_tp_spawn {
            save_current_store();
            let _ = execute_teleport(
                &event.player,
                &server,
                &spawn_loc,
                "§6§l✦ KRONOS",
                "§aДобро пожаловать!",
                &cfg,
                false,
                false,
            );
        }
        event
    }
}

struct LeaveHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(&self, _server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let p_key = normalize_key(&event.player.get_name());
        with_state(|st| {
            st.tpa_requests.remove(&p_key);
            st.pending_forms.retain(|_, f| f.player_key != p_key);
        });
        event
    }
}

struct BedrockFormHandler;
impl EventHandler<BedrockFormResponseEvent> for BedrockFormHandler {
    fn handle(
        &self,
        server: Server,
        event: BedrockFormResponseEventData,
    ) -> BedrockFormResponseEventData {
        let pending_opt = with_state(|st| st.pending_forms.remove(&event.form_id));
        if let (Some(pending), Some(raw_response)) = (pending_opt, event.response_data.as_deref()) {
            let p_key = normalize_key(&event.player.get_name());
            if pending.player_key == p_key {
                if let Ok(button_idx) = raw_response.trim().parse::<usize>() {
                    if let Some(action) = pending.actions.into_iter().nth(button_idx) {
                        execute_bedrock_action(&event.player, &server, action);
                    } else {
                        play_error_sound(&event.player);
                    }
                }
            }
        }
        event
    }
}

// =============================================================================
// Plugin Entrypoint
// =============================================================================

pub struct KronosEssentialsPlugin;

impl Plugin for KronosEssentialsPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kronos-essentials".into(),
            version: "2.0.0".into(),
            authors: vec!["UncleBug".into()],
            description: "Homes, Shared Homes, Spawn, Warps, Safe RTP, TPA, Anti-Combat PvP Timer, and Interactive Menus for Pumpkin 26.3".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_storage(context.get_data_folder());

        context.register_permission(&Permission {
            node: PERM_PLAYER.to_string(),
            description: "Access to KRONOS Essentials player teleport and home commands".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        context.register_permission(&Permission {
            node: PERM_ADMIN.to_string(),
            description: "Access to KRONOS Essentials admin spawn and warp management".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: vec![],
        })?;

        commands::register_all(&context);

        context.register_event_handler::<EntityDamageByEntityEvent, _>(
            PvPCombatHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<EntityDamageEvent, _>(
            DamageCombatHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<PlayerDeathEvent, _>(
            DeathHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<PlayerJoinEvent, _>(
            JoinHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<PlayerLeaveEvent, _>(
            LeaveHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<BedrockFormResponseEvent, _>(
            BedrockFormHandler,
            EventPriority::Normal,
            false,
        )?;

        tracing::info!(
            "KRONOS Essentials v2.0.0 loaded (pumpkin_plugin_api, Anti-Combat 15s, Safe RTP, Shared Homes)."
        );
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_current_store();
        tracing::info!("KRONOS Essentials unloaded and essentials_store.json saved.");
        Ok(())
    }
}

register_plugin!(KronosEssentialsPlugin);

// =============================================================================
// Unit Tests (Pure Domain Logic)
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::effects::{
        is_block_hazardous, is_passable_space_props, is_standable_ground_props, sample_ring_coords,
    };
    use crate::models::{
        EssentialsConfig, EssentialsState, EssentialsStore, LocationPoint,
    };
    use crate::storage::{
        list_shared_homes_for, parse_store_json, resolve_shared_home, share_home, unshare_home,
        upsert_home, SetHomeOutcome, ShareOutcome, UnshareOutcome,
    };

    #[test]
    fn rtp_rejects_lava_water_leaves_and_accepts_grass_block() {
        assert!(is_block_hazardous("minecraft:lava"));
        assert!(is_block_hazardous("minecraft:water"));
        assert!(is_block_hazardous("minecraft:cactus"));
        assert!(!is_block_hazardous("minecraft:grass_block"));

        assert!(is_standable_ground_props(
            true,
            false,
            false,
            true,
            "minecraft:grass_block",
            true
        ));
        assert!(!is_standable_ground_props(
            true,
            false,
            false,
            true,
            "minecraft:oak_leaves",
            true
        ));
        assert!(!is_standable_ground_props(
            false,
            true,
            false,
            false,
            "minecraft:lava",
            true
        ));
        assert!(is_passable_space_props(
            true,
            false,
            false,
            true,
            "minecraft:air"
        ));
        assert!(!is_passable_space_props(
            false,
            true,
            false,
            true,
            "minecraft:lava"
        ));
    }

    #[test]
    fn rtp_ring_coords_stay_within_configured_bounds() {
        let (x, z) = sample_ring_coords(100.0, -50.0, 250, 1500, 4242, 18000);
        let dist = (((x as f64 - 100.0).powi(2)) + ((z as f64 + 50.0).powi(2))).sqrt();
        assert!(dist >= 249.0 && dist <= 1501.0);
    }

    #[test]
    fn anti_combat_timer_blocks_for_15_seconds_then_expires() {
        let mut state = EssentialsState::new(
            ".".to_string(),
            EssentialsConfig::default(),
            EssentialsStore::default(),
        );
        let (secs, notified) = state.tag_combat("Steve", 10_000);
        assert_eq!(secs, 15);
        assert!(notified);
        assert_eq!(state.combat_remaining_secs("steve", 10_500), Some(15));
        assert_eq!(state.combat_remaining_secs("Steve", 20_000), Some(5));
        assert_eq!(state.combat_remaining_secs("Steve", 25_001), None);
    }

    #[test]
    fn shared_homes_grant_list_resolve_and_revoke_access() {
        let mut store = EssentialsStore::default();
        let loc = LocationPoint::rounded("minecraft:overworld".into(), 12.0, 64.0, -34.0, 0.0, 0.0);
        assert!(matches!(
            upsert_home(&mut store, "Alice", "base", loc, 3, false, 1000),
            SetHomeOutcome::Created { .. }
        ));

        assert_eq!(
            share_home(&mut store, "Alice", "base", "Bob", 5),
            ShareOutcome::Shared {
                home: "base".into(),
                friend: "Bob".into()
            }
        );

        let shared_for_bob = list_shared_homes_for(&store, "bob");
        assert_eq!(shared_for_bob.len(), 1);
        assert_eq!(shared_for_bob[0].home.name, "base");

        assert!(resolve_shared_home(&mut store, "Bob", "Alice", "base").is_some());
        assert!(resolve_shared_home(&mut store, "Charlie", "Alice", "base").is_none());

        assert_eq!(
            unshare_home(&mut store, "Alice", "base", "Bob"),
            UnshareOutcome::Unshared {
                home: "base".into(),
                friend: "Bob".into()
            }
        );
        assert!(resolve_shared_home(&mut store, "Bob", "Alice", "base").is_none());
    }

    #[test]
    fn legacy_essentials_store_migrates_cleanly() {
        let raw = r#"{
            "homes": { "UncleBug": { "base": { "world": "minecraft:overworld", "x": 10.0, "y": 70.0, "z": -20.0, "yaw": 0.0, "pitch": 0.0 } } },
            "warps": {},
            "spawn": { "world": "minecraft:overworld", "x": 0.5, "y": 65.0, "z": 0.5, "yaw": 0.0, "pitch": 0.0 },
            "last_locations": {}
        }"#;
        let parsed = parse_store_json(raw).expect("legacy store must parse");
        let homes = crate::storage::list_player_homes(&parsed, "UncleBug");
        assert_eq!(homes.len(), 1);
        assert_eq!(homes[0].name, "base");
    }
}
