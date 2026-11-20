pub mod commands;
pub mod hierarchy;
pub mod menu;
pub mod models;
pub mod storage;
pub mod tab_chat;

use pumpkin_plugin_api::{
    events::{
        BedrockFormResponseEvent, EventHandler, EventPriority, PlayerChatEvent, PlayerJoinEvent,
        PlayerLeaveEvent, ServerTickEndEvent,
    },
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    wit::pumpkin::plugin::event::{
        BedrockFormResponseEventData, PlayerChatEventData, PlayerJoinEventData,
        PlayerLeaveEventData, ServerTickEndEventData,
    },
    Context, Plugin, PluginMetadata, Server,
};

use crate::commands::{execute_bedrock_action, PERM_ADMIN, PERM_PLAYER, PERM_STAFF};
use crate::hierarchy::{expire_temp_ranks, normalize_key};
use crate::storage::{
    ensure_profile, get_or_default_profile, init_storage, now_ms, save_current_store, with_state,
};
use crate::tab_chat::{
    announce_join_flair, find_online_player_ci, format_chat_line, legacy, play_error_sound,
    refresh_all_headers_footers, refresh_all_online_tabs, refresh_online_player_by_name,
    refresh_player_display, sync_player_native_permissions,
};

// =============================================================================
// Event Listeners (Tablist, Chat Formatting, Temp-Rank Expiration, Bedrock Forms)
// =============================================================================

struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let p_name = event.player.get_name();
        let now = now_ms();

        let (config, profile, anim_frame, should_save) = with_state(|st| {
            let key = normalize_key(&p_name);
            let existed = st.store.profiles.contains_key(&key);
            let prof = ensure_profile(&mut st.store, &p_name, now).clone();
            let expired = expire_temp_ranks(&mut st.store, &mut st.dirty, now);
            let updated_prof = st.store.profiles.get(&key).cloned().unwrap_or(prof);
            if !existed {
                st.dirty = true;
            }
            (
                st.config.clone(),
                updated_prof,
                st.anim_frame,
                !existed || !expired.is_empty(),
            )
        });

        if should_save {
            save_current_store();
        }

        let online_count = server.get_player_count().max(1);
        let max_players = server.get_max_players();

        sync_player_native_permissions(&event.player, &config, &profile);
        refresh_player_display(
            &event.player,
            &config,
            &profile,
            online_count,
            max_players,
            anim_frame,
        );
        refresh_all_headers_footers(&server);
        announce_join_flair(&server, &event.player, &config, &profile);

        event
    }
}

struct LeaveHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(&self, server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let p_key = normalize_key(&event.player.get_name());
        with_state(|st| {
            st.pending_forms.retain(|_, f| f.player_key != p_key);
        });
        refresh_all_headers_footers(&server);
        event
    }
}

struct ChatHandler;
impl EventHandler<PlayerChatEvent> for ChatHandler {
    fn handle(&self, server: Server, mut event: PlayerChatEventData) -> PlayerChatEventData {
        if event.cancelled {
            return event;
        }

        let p_name = event.player.get_name();
        let now = now_ms();
        let formatted = with_state(|st| {
            let prof = get_or_default_profile(&st.store, &p_name, now);
            format_chat_line(&st.config, &prof, &event.message)
        });

        // Cancel default raw chat broadcast and deliver formatted TextComponent with prefix/suffix/colors
        event.cancelled = true;
        if event.recipients.is_empty() {
            for recipient in server.get_all_players() {
                recipient.send_system_message(legacy(&formatted), false);
            }
        } else {
            for recipient in &event.recipients {
                recipient.send_system_message(legacy(&formatted), false);
            }
        }
        tracing::info!("[Chat] {}", formatted);

        event
    }
}

struct TickHandler;
impl EventHandler<ServerTickEndEvent> for TickHandler {
    fn handle(&self, server: Server, event: ServerTickEndEventData) -> ServerTickEndEventData {
        let now = now_ms();

        let (should_refresh_tab, expired_keys, should_autosave) = with_state(|st| {
            st.tick_counter = st.tick_counter.wrapping_add(1);

            let refresh_tab = st.tick_counter % 40 == 0;
            if refresh_tab {
                st.anim_frame = st.anim_frame.wrapping_add(1);
            }

            let expired = if st.tick_counter % 100 == 0 {
                expire_temp_ranks(&mut st.store, &mut st.dirty, now)
            } else {
                Vec::new()
            };

            let autosave_ms = st.config.autosave_interval_seconds.max(15) * 1000;
            let autosave = st.dirty && now.saturating_sub(st.last_autosave_ms) >= autosave_ms;

            (refresh_tab, expired, autosave)
        });

        if !expired_keys.is_empty() {
            save_current_store();
            for (player_key, old_rank) in expired_keys {
                refresh_online_player_by_name(&server, &player_key);
                if let Some(p) = find_online_player_ci(&server, &player_key) {
                    p.send_system_message(
                        legacy(&format!(
                            "§6§l✦ KRONOS RANKS §8» §eСрок действия ранга §f{} §eистёк.",
                            old_rank
                        )),
                        false,
                    );
                }
            }
            refresh_all_online_tabs(&server);
        } else if should_refresh_tab {
            refresh_all_headers_footers(&server);
        }

        if should_autosave {
            save_current_store();
        }

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

pub struct KronosRanksPlugin;

impl Plugin for KronosRanksPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kronos-ranks".into(),
            version: "1.0.0".into(),
            authors: vec!["UncleBug".into()],
            description: "20-Rank Dual-Track Hierarchy, Animated Tablist, Formatted Chat, Staff Chat (/sc), HR Workflow, and Cross-Plugin Bridge for Pumpkin 26.3".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_storage(context.get_data_folder());

        context.register_permission(&Permission {
            node: PERM_PLAYER.to_string(),
            description: "Access to KRONOS Ranks showcase, profile, and application commands".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        context.register_permission(&Permission {
            node: PERM_STAFF.to_string(),
            description: "Access to KRONOS Staff Chat (/sc), HR, and Audit tools".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Two),
            children: vec![],
        })?;

        context.register_permission(&Permission {
            node: PERM_ADMIN.to_string(),
            description: "Full administrative access to KRONOS Ranks management".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: vec![],
        })?;

        commands::register_all(&context);

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
        context.register_event_handler::<PlayerChatEvent, _>(
            ChatHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<ServerTickEndEvent, _>(
            TickHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<BedrockFormResponseEvent, _>(
            BedrockFormHandler,
            EventPriority::Normal,
            false,
        )?;

        tracing::info!(
            "KRONOS Ranks v1.0.0 loaded (pumpkin_plugin_api, 20 ranks, Dual-Track, Tablist, Chat & /sc)."
        );
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_current_store();
        tracing::info!("KRONOS Ranks unloaded: kronos_ranks.json and kronos_ranks_bridge.json saved.");
        Ok(())
    }
}

register_plugin!(KronosRanksPlugin);

// =============================================================================
// Unit Tests (Pure Domain Logic)
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::hierarchy::{
        assign_rank, can_assign_rank, effective_perks, expire_temp_ranks, has_permission,
        is_staff_member, primary_rank, remove_rank_track, reset_all_ranks,
    };
    use crate::models::{parse_duration_seconds, PlayerRankProfile, RanksStore};
    use crate::storage::{build_bridge_map, default_config};
    use crate::tab_chat::{
        build_tab_header_footer, build_tab_name, format_chat_line, format_staff_chat_line,
        tab_sort_order,
    };

    #[test]
    fn config_contains_exact_20_ranks_with_5_departments() {
        let cfg = default_config();
        assert_eq!(cfg.ranks.len(), 20);
        assert_eq!(cfg.donate_ranks().len(), 10);
        assert_eq!(cfg.staff_ranks().len(), 9);
        assert!(cfg.find("player").is_some());
        assert!(cfg.find("kronos").is_some());
        assert!(cfg.find("hr_curator").is_some());
        assert!(cfg.find("owner").is_some());
    }

    #[test]
    fn dual_track_keeps_donate_perks_while_showing_staff_prefix() {
        let cfg = default_config();
        let mut store = RanksStore::default();
        let mut dirty = false;

        assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "Steve", "kronos", None, 1000)
            .expect("assign kronos");
        assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "Steve", "trainee", None, 1000)
            .expect("assign trainee");

        let prof = store.profiles.get("steve").expect("profile exists");
        assert_eq!(primary_rank(&cfg, prof).id, "trainee");
        assert_eq!(prof.donate_rank, "kronos");
        assert_eq!(prof.staff_rank.as_deref(), Some("trainee"));
        assert!(is_staff_member(&cfg, prof));

        let perks = effective_perks(&cfg, prof);
        assert_eq!(perks.max_homes, 50);
        assert_eq!(perks.max_regions, 30);
        assert_eq!(perks.max_region_volume, 3_500_000);
        assert!(has_permission(&cfg, prof, "kronos.guard.warn"));
        assert!(has_permission(&cfg, prof, "kronos.essentials.home.50"));

        let bridge = build_bridge_map(&cfg, &store);
        let snap = bridge.get("steve").expect("bridge entry");
        assert_eq!(snap.primary_rank, "trainee");
        assert_eq!(snap.donate_rank, "kronos");
        assert_eq!(snap.max_homes, 50);
        assert_eq!(snap.weight, 100);
    }

    #[test]
    fn hr_curator_department_boundaries_and_reset_guards() {
        let cfg = default_config();
        let mut store = RanksStore::default();
        let mut dirty = false;

        assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "HRLead", "hr_curator", None, 1000)
            .expect("assign hr_curator");

        let hr_prof = store.profiles.get("hrlead").unwrap().clone();
        let moder_def = cfg.find("moder").unwrap();
        let tech_def = cfg.find("tech_admin").unwrap();
        let vip_def = cfg.find("vip").unwrap();

        assert!(can_assign_rank(&cfg, Some(&hr_prof), false, None, moder_def).is_ok());
        assert!(can_assign_rank(&cfg, Some(&hr_prof), false, None, tech_def).is_err());
        assert!(can_assign_rank(&cfg, Some(&hr_prof), false, None, vip_def).is_err());

        assign_rank(&cfg, &mut store, &mut dirty, Some("hrlead"), "HRLead", false, "ModGuy", "moder", None, 2000)
            .expect("hr hires moder");
        assert!(remove_rank_track(&cfg, &mut store, &mut dirty, Some("hrlead"), "HRLead", false, "ModGuy", "staff", 3000).is_ok());
        assert!(reset_all_ranks(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "ModGuy", 4000).is_ok());
    }

    #[test]
    fn temp_rank_expiration_and_cyrillic_durations() {
        assert_eq!(parse_duration_seconds("30д"), Ok(Some(30 * 86_400)));
        assert_eq!(parse_duration_seconds("12ч"), Ok(Some(12 * 3600)));
        assert_eq!(parse_duration_seconds("15м"), Ok(Some(15 * 60)));
        assert_eq!(parse_duration_seconds("perm"), Ok(None));

        let cfg = default_config();
        let mut store = RanksStore::default();
        let mut dirty = false;

        assign_rank(&cfg, &mut store, &mut dirty, None, "CONSOLE", true, "TempVip", "vip", Some(60), 10_000)
            .expect("temp vip");
        let expired_early = expire_temp_ranks(&mut store, &mut dirty, 50_000);
        assert!(expired_early.is_empty());

        let expired_now = expire_temp_ranks(&mut store, &mut dirty, 75_000);
        assert_eq!(expired_now.len(), 1);
        assert_eq!(store.profiles.get("tempvip").unwrap().donate_rank, "player");
    }

    #[test]
    fn tab_order_and_chat_formatting_respect_rank_tiers() {
        let cfg = default_config();
        let mut owner_prof = PlayerRankProfile::new("UncleBug", 1000);
        owner_prof.staff_rank = Some("owner".into());

        let mut vip_prof = PlayerRankProfile::new("Alex", 1000);
        vip_prof.donate_rank = "vip".into();

        let player_prof = PlayerRankProfile::new("Newbie", 1000);

        assert!(tab_sort_order(&cfg, &owner_prof) > tab_sort_order(&cfg, &vip_prof));
        assert!(tab_sort_order(&cfg, &vip_prof) > tab_sort_order(&cfg, &player_prof));
        assert!(build_tab_name(&cfg, &owner_prof).contains("UncleBug"));

        let vip_chat = format_chat_line(&cfg, &vip_prof, "&aПривет всем!");
        assert!(vip_chat.contains("§aПривет всем!"));

        let newbie_chat = format_chat_line(&cfg, &player_prof, "&cНельзя цвет");
        assert!(newbie_chat.contains("&cНельзя цвет"));

        let sc = format_staff_chat_line(&cfg, &owner_prof, "Проверьте жалобу");
        assert!(sc.contains("[§b§lSC§8]") && sc.contains("UncleBug"));

        let (h0, f0) = build_tab_header_footer(&cfg, &owner_prof, 42, 200, 28, 0);
        let (h1, _) = build_tab_header_footer(&cfg, &owner_prof, 42, 200, 28, 1);
        assert_ne!(h0, h1);
        assert!(h0.contains("42") && f0.contains("28ms"));
    }
}
