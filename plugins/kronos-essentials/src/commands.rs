use pumpkin_plugin_api::{
    command::{
        ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType,
    },
    commands::CommandHandler,
    wit::pumpkin::plugin::command::Arg,
    Context, Player, Server,
};

use crate::effects::{
    capture_player_location, execute_teleport, find_safe_rtp_location, legacy, play_chime_sound,
    play_click_sound, play_error_sound, play_success_sound, resolve_world,
};
use crate::menu::{
    clickable_run, open_homes_menu, send_shared_homes_menu, send_tpa_prompt, send_warps_menu,
    HomesMenuSnapshot,
};
use crate::models::{
    normalize_key, sanitize_point_name, BedrockMenuAction, EssentialsState, LocationPoint,
    TpaRequest, DEFAULT_HOME_NAME,
};
use crate::storage::{
    delete_home, delete_warp, list_player_homes, list_shared_homes_for, list_warps, now_ms,
    resolve_own_home, resolve_shared_home, resolve_warp, save_current_store, set_last_location,
    share_home, toggle_favorite_home, unshare_home, upsert_home, upsert_warp, with_state,
    SetHomeOutcome, ShareOutcome, UnshareOutcome,
};

pub const PERM_PLAYER: &str = "kronos-essentials:player";
pub const PERM_ADMIN: &str = "kronos-essentials:admin";

fn extract_args(args: &ConsumedArgs) -> String {
    match args.get_value("args") {
        Arg::Simple(s) | Arg::Msg(s) => s,
        _ => String::new(),
    }
}

fn require_player(sender: &CommandSender) -> Option<Player> {
    match sender.as_player() {
        Some(player) => Some(player),
        None => {
            sender.send_error(legacy("§cКоманда доступна только игрокам!"));
            None
        }
    }
}

/// Checks if the player is under the 15-second Anti-Combat PvP lock.
/// Never holds a Mutex lock across WIT host calls.
pub fn is_combat_blocked(player: &Player) -> bool {
    let p_name = player.get_name();
    let remaining = with_state(|st| st.combat_remaining_secs(&p_name, now_ms()));
    if let Some(sec) = remaining {
        let msg = format!("§c⚔ Вы в бою! Телепортация заблокирована на {}с", sec);
        player.send_system_message(legacy(&msg), false);
        player.show_actionbar(legacy(&msg));
        play_error_sound(player);
        return true;
    }
    false
}

fn check_tp_cooldown(player: &Player) -> bool {
    if player.has_permission(PERM_ADMIN) {
        return false;
    }
    let p_name = player.get_name();
    let remaining = with_state(|st| {
        EssentialsState::cooldown_remaining_secs(&mut st.tp_cooldowns, &p_name, now_ms())
    });
    if let Some(sec) = remaining {
        player.send_system_message(
            legacy(&format!(
                "§c⏳ Подождите §e{}с §cперед следующей телепортацией.",
                sec
            )),
            false,
        );
        play_error_sound(player);
        return true;
    }
    false
}

fn apply_tp_cooldown(player: &Player) {
    if player.has_permission(PERM_ADMIN) {
        return;
    }
    let p_name = player.get_name();
    let now = now_ms();
    with_state(|st| {
        let pdata = st
            .store
            .players
            .get(&normalize_key(&p_name))
            .cloned()
            .unwrap_or_default();
        let profile = st.config.effective_rank_for(&pdata);
        EssentialsState::set_cooldown(&mut st.tp_cooldowns, &p_name, profile.cooldown_seconds, now);
    });
}

fn record_back_location(player: &Player) {
    let p_name = player.get_name();
    let loc = capture_player_location(player);
    with_state(|st| {
        set_last_location(&mut st.store, &p_name, loc);
    });
}

fn build_homes_snapshot(player: &Player) -> HomesMenuSnapshot {
    let p_name = player.get_name();
    let is_admin = player.has_permission(PERM_ADMIN);
    let now = now_ms();

    with_state(|st| {
        st.cleanup_expired(now);
        let p_key = normalize_key(&p_name);
        let pdata = st.store.players.get(&p_key).cloned().unwrap_or_default();
        let profile = st.config.effective_rank_for(&pdata);
        let has_back = pdata.last_location.is_some() || pdata.death_location.is_some();

        HomesMenuSnapshot {
            player_name: p_name.clone(),
            homes: list_player_homes(&st.store, &p_name),
            shared_with_me: list_shared_homes_for(&st.store, &p_name),
            max_homes: profile.max_homes,
            unlimited: profile.unlimited_homes || is_admin,
            combat_remaining: st.combat_remaining_secs(&p_name, now),
            incoming_tpa: st.tpa_requests.get(&p_key).cloned(),
            warps_count: st.store.warps.len(),
            has_back,
        }
    })
}

pub fn execute_bedrock_action(player: &Player, server: &Server, action: BedrockMenuAction) {
    match action {
        BedrockMenuAction::TeleportHome(name) => {
            teleport_to_own_home(player, server, Some(&name));
        }
        BedrockMenuAction::TeleportShared { owner, home } => {
            teleport_to_shared_home(player, server, &owner, &home);
        }
        BedrockMenuAction::RunSpawn => {
            run_spawn_teleport(player, server);
        }
        BedrockMenuAction::RunRtp => {
            run_rtp_teleport(player, server);
        }
        BedrockMenuAction::RunBack => {
            run_back_teleport(player, server);
        }
        BedrockMenuAction::OpenWarps => {
            let warps = with_state(|st| list_warps(&st.store));
            send_warps_menu(player, &warps);
        }
    }
}

// =============================================================================
// Shared Teleport Helpers
// =============================================================================

fn teleport_to_own_home(player: &Player, server: &Server, requested: Option<&str>) {
    if is_combat_blocked(player) || check_tp_cooldown(player) {
        return;
    }

    let p_name = player.get_name();
    let (home_opt, cfg) = with_state(|st| {
        (
            resolve_own_home(&mut st.store, &p_name, requested),
            st.config.clone(),
        )
    });

    let Some(home) = home_opt else {
        play_error_sound(player);
        let msg = match requested {
            Some(name) => format!(
                "§cУ вас нет точки дома '§e{}§c'. Создайте: §e/sethome {}",
                name, name
            ),
            None => "§cУ вас пока нет сохранённых домов. Создайте: §e/sethome home".to_string(),
        };
        player.send_system_message(legacy(&msg), false);
        return;
    };

    record_back_location(player);
    match execute_teleport(
        player,
        server,
        &home.location,
        "§6§l✦ ДОМ",
        &format!("§e{}", home.name),
        &cfg,
        false,
        true,
    ) {
        Ok(()) => {
            apply_tp_cooldown(player);
            save_current_store();
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS §8» §aТелепортация на точку дома §e{}§a!",
                    home.name
                )),
                false,
            );
        }
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(legacy(&format!("§cОшибка телепортации: {}", err)), false);
        }
    }
}

fn teleport_to_shared_home(player: &Player, server: &Server, owner: &str, home_name: &str) {
    if is_combat_blocked(player) || check_tp_cooldown(player) {
        return;
    }

    let p_name = player.get_name();
    let (home_opt, cfg) = with_state(|st| {
        (
            resolve_shared_home(&mut st.store, &p_name, owner, home_name),
            st.config.clone(),
        )
    });

    let Some(home) = home_opt else {
        play_error_sound(player);
        player.send_system_message(
            legacy(&format!(
                "§cУ вас нет доступа к дому §e{}:{} §cили он не существует.",
                owner, home_name
            )),
            false,
        );
        return;
    };

    record_back_location(player);
    match execute_teleport(
        player,
        server,
        &home.location,
        "§d§l👥 ОБЩИЙ ДОМ",
        &format!("§f{}:§d{}", owner, home.name),
        &cfg,
        false,
        true,
    ) {
        Ok(()) => {
            apply_tp_cooldown(player);
            save_current_store();
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS §8» §aТелепортация в общий дом §f{}§7:§d{}§a!",
                    owner, home.name
                )),
                false,
            );
        }
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(legacy(&format!("§cОшибка телепортации: {}", err)), false);
        }
    }
}

fn handle_share_home_action(player: &Player, server: &Server, home_name: &str, friend_name: &str) {
    let Some(clean_home) = sanitize_point_name(home_name) else {
        play_error_sound(player);
        player.send_system_message(
            legacy("§cНекорректное имя дома! Используйте буквы, цифры, '_' или '-'."),
            false,
        );
        return;
    };
    let clean_friend = friend_name.trim();
    if clean_friend.is_empty() || clean_friend.len() > 24 {
        play_error_sound(player);
        player.send_system_message(
            legacy("§eИспользование: §b/sharehome <дом> <ник_друга>"),
            false,
        );
        return;
    }

    let p_name = player.get_name();
    let is_admin = player.has_permission(PERM_ADMIN);

    let outcome = with_state(|st| {
        let pdata = st
            .store
            .players
            .get(&normalize_key(&p_name))
            .cloned()
            .unwrap_or_default();
        let profile = st.config.effective_rank_for(&pdata);
        let limit = if is_admin {
            100
        } else {
            profile.max_shared_per_home.max(2)
        };
        share_home(&mut st.store, &p_name, &clean_home, clean_friend, limit)
    });

    match outcome {
        ShareOutcome::Shared { home, friend } => {
            save_current_store();
            play_success_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS §8» §aВы открыли доступ к дому §e{} §aигроку §d{}§a! §7(Отмена: /home unshare {} {})",
                    home, friend, home, friend
                )),
                false,
            );

            // Notify friend if currently online
            if let Some(friend_player) = server.get_player_by_name(&friend) {
                play_chime_sound(&friend_player);
                let msg = legacy(&format!(
                    "§6§l✦ KRONOS §8» §e{} §aподелился с вами домом §d{}§a! ",
                    p_name, home
                )).add_child(clickable_run(
                    "§a§l[ТП]",
                    &format!("/home {}:{}", p_name, home),
                    &format!("§aТелепортироваться в общий дом §f{}:{}", p_name, home),
                ));
                friend_player.send_system_message(msg, false);
            }
        }
        ShareOutcome::AlreadyShared { home, friend } => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§eИгрок §f{} §eуже имеет доступ к дому §6{}§e.",
                    friend, home
                )),
                false,
            );
        }
        ShareOutcome::HomeNotFound => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!("§cТочка дома '§e{}§c' не найдена!", clean_home)),
                false,
            );
        }
        ShareOutcome::CannotShareWithSelf => {
            play_error_sound(player);
            player.send_system_message(
                legacy("§cНельзя поделиться домом с самим собой!"),
                false,
            );
        }
        ShareOutcome::LimitReached { max_shared } => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§cДостигнут лимит общих друзей для этого дома (§e{}§c).",
                    max_shared
                )),
                false,
            );
        }
    }
}

fn handle_unshare_home_action(player: &Player, home_name: &str, friend_name: &str) {
    let p_name = player.get_name();
    let outcome = with_state(|st| unshare_home(&mut st.store, &p_name, home_name, friend_name));

    match outcome {
        UnshareOutcome::Unshared { home, friend } => {
            save_current_store();
            play_success_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS §8» §eДоступ игрока §f{} §eк дому §6{} §eотозван.",
                    friend, home
                )),
                false,
            );
        }
        UnshareOutcome::HomeNotFound => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!("§cТочка дома '§e{}§c' не найдена!", home_name)),
                false,
            );
        }
        UnshareOutcome::FriendNotShared => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§cУ игрока §f{} §cне было доступа к дому §e{}§c.",
                    friend_name, home_name
                )),
                false,
            );
        }
    }
}

fn run_spawn_teleport(player: &Player, server: &Server) {
    if is_combat_blocked(player) || check_tp_cooldown(player) {
        return;
    }

    let (spawn_loc, cfg) = with_state(|st| {
        (
            st.store
                .spawn
                .clone()
                .unwrap_or_else(LocationPoint::default_spawn),
            st.config.clone(),
        )
    });

    record_back_location(player);
    match execute_teleport(
        player,
        server,
        &spawn_loc,
        "§6§l✦ СПАВН",
        "§eДобро пожаловать на спавн",
        &cfg,
        false,
        false,
    ) {
        Ok(()) => {
            apply_tp_cooldown(player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS §8» §aВы телепортированы на спавн сервера!"),
                false,
            );
        }
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(legacy(&format!("§cОшибка: {}", err)), false);
        }
    }
}

fn run_back_teleport(player: &Player, server: &Server) {
    if is_combat_blocked(player) || check_tp_cooldown(player) {
        return;
    }

    let p_name = player.get_name();
    let (target_loc, cfg) = with_state(|st| {
        let loc = st
            .store
            .players
            .get(&normalize_key(&p_name))
            .and_then(|p| p.last_location.clone().or_else(|| p.death_location.clone()));
        (loc, st.config.clone())
    });

    let Some(loc) = target_loc else {
        play_error_sound(player);
        player.send_system_message(
            legacy("§cУ вас нет сохранённой предыдущей локации!"),
            false,
        );
        return;
    };

    record_back_location(player);
    match execute_teleport(
        player,
        server,
        &loc,
        "§d§l↩ ВОЗВРАТ",
        "§7Предыдущая позиция",
        &cfg,
        false,
        true,
    ) {
        Ok(()) => {
            apply_tp_cooldown(player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS §8» §aВы вернулись на предыдущую локацию!"),
                false,
            );
        }
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(legacy(&format!("§cОшибка: {}", err)), false);
        }
    }
}

fn run_rtp_teleport(player: &Player, server: &Server) {
    if is_combat_blocked(player) {
        return;
    }

    let p_name = player.get_name();
    let is_admin = player.has_permission(PERM_ADMIN);
    let now = now_ms();

    let (cd_rem, cfg, spawn_center, mut seed, rtp_cd_secs) = with_state(|st| {
        let rem = if is_admin {
            None
        } else {
            EssentialsState::cooldown_remaining_secs(&mut st.rtp_cooldowns, &p_name, now)
        };
        let pdata = st
            .store
            .players
            .get(&normalize_key(&p_name))
            .cloned()
            .unwrap_or_default();
        let profile = st.config.effective_rank_for(&pdata);
        let center = st
            .store
            .spawn
            .as_ref()
            .map(|s| (s.x, s.z))
            .unwrap_or((0.0, 0.0));
        let seed_val = st.next_u64();
        (
            rem,
            st.config.clone(),
            center,
            seed_val,
            profile.rtp_cooldown_seconds,
        )
    });

    if let Some(sec) = cd_rem {
        play_error_sound(player);
        player.send_system_message(
            legacy(&format!(
                "§c⏳ Перезарядка /rtp: подождите §e{}с§c.",
                sec
            )),
            false,
        );
        return;
    }

    let Some(world) = resolve_world(server, &cfg.rtp.world) else {
        play_error_sound(player);
        player.send_system_message(legacy("§cОшибка: целевой мир для RTP недоступен!"), false);
        return;
    };

    let Some(safe_loc) = find_safe_rtp_location(&world, spawn_center.0, spawn_center.1, &cfg.rtp, &mut seed) else {
        play_error_sound(player);
        player.send_system_message(
            legacy("§cНе удалось найти безопасную поверхность для RTP (океан/лава/граница). Попробуйте ещё раз — кулдаун не потрачен."),
            false,
        );
        return;
    };

    with_state(|st| {
        st.prng_seed = seed;
        if !is_admin {
            EssentialsState::set_cooldown(&mut st.rtp_cooldowns, &p_name, rtp_cd_secs, now_ms());
        }
    });

    record_back_location(player);

    // Move already-resolved world directly into teleport (no second unsafe guess!)
    player.teleport(
        (safe_loc.x, safe_loc.y, safe_loc.z),
        Some(safe_loc.yaw),
        Some(safe_loc.pitch),
        world,
    );
    crate::effects::play_teleport_effects(
        player,
        &safe_loc,
        "§a§l☄ RTP",
        &format!("§7{:.0}, {:.0}, {:.0}", safe_loc.x, safe_loc.y, safe_loc.z),
        &cfg,
        true,
    );

    player.send_system_message(
        legacy(&format!(
            "§6§l✦ KRONOS §8» §aБезопасная случайная телепортация на §eX:{:.0} Y:{:.0} Z:{:.0}§a!",
            safe_loc.x, safe_loc.y, safe_loc.z
        )),
        false,
    );
}

// =============================================================================
// Command Handlers
// =============================================================================

struct SetHomeCmd;
impl CommandHandler for SetHomeCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };

        let raw = extract_args(&args);
        let requested = raw
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or(DEFAULT_HOME_NAME);

        let Some(home_name) = sanitize_point_name(requested) else {
            play_error_sound(&player);
            sender.send_error(legacy(
                "§cИмя дома может содержать только буквы, цифры, '_' и '-' (до 24 символов).",
            ));
            return Ok(0);
        };

        let p_name = player.get_name();
        let is_admin = player.has_permission(PERM_ADMIN);
        let loc = capture_player_location(&player);
        let now = now_ms();

        let outcome = with_state(|st| {
            let pdata = st
                .store
                .players
                .get(&normalize_key(&p_name))
                .cloned()
                .unwrap_or_default();
            let profile = st.config.effective_rank_for(&pdata);
            upsert_home(
                &mut st.store,
                &p_name,
                &home_name,
                loc.clone(),
                profile.max_homes,
                profile.unlimited_homes || is_admin,
                now,
            )
        });

        match outcome {
            SetHomeOutcome::Created { name, total } => {
                save_current_store();
                play_success_sound(&player);
                sender.send_system_message(legacy(&format!(
                    "§6§l✦ KRONOS §8» §aТочка дома §e{} §aсоздана! §7({:.1}, {:.1}, {:.1} | Всего: {})",
                    name, loc.x, loc.y, loc.z, total
                )));
            }
            SetHomeOutcome::Updated { name, .. } => {
                save_current_store();
                play_success_sound(&player);
                sender.send_system_message(legacy(&format!(
                    "§6§l✦ KRONOS §8» §aКоординаты дома §e{} §aобновлены! §7({:.1}, {:.1}, {:.1})",
                    name, loc.x, loc.y, loc.z
                )));
            }
            SetHomeOutcome::LimitReached { max_homes } => {
                play_error_sound(&player);
                sender.send_error(legacy(&format!(
                    "§cДостигнут лимит точек дома (§e{}§c). Удалите лишний дом: §e/homes",
                    max_homes
                )));
            }
        }

        Ok(0)
    }
}

struct HomeCmd;
impl CommandHandler for HomeCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };

        let raw = extract_args(&args);
        let parts: Vec<&str> = raw.split_whitespace().collect();

        if parts.is_empty() {
            let p_name = player.get_name();
            let has_homes = with_state(|st| !list_player_homes(&st.store, &p_name).is_empty());
            if has_homes {
                teleport_to_own_home(&player, &server, None);
            } else {
                let snap = build_homes_snapshot(&player);
                open_homes_menu(&player, &snap);
            }
            return Ok(0);
        }

        match parts[0].to_ascii_lowercase().as_str() {
            "list" | "menu" | "gui" => {
                let snap = build_homes_snapshot(&player);
                open_homes_menu(&player, &snap);
            }
            "share" => {
                if parts.len() < 3 {
                    play_error_sound(&player);
                    sender.send_error(legacy(
                        "§eИспользование: §b/home share <название_дома> <ник_друга>",
                    ));
                    return Ok(0);
                }
                handle_share_home_action(&player, &server, parts[1], parts[2]);
            }
            "unshare" | "revoke" => {
                if parts.len() < 3 {
                    play_error_sound(&player);
                    sender.send_error(legacy(
                        "§eИспользование: §b/home unshare <название_дома> <ник_друга>",
                    ));
                    return Ok(0);
                }
                handle_unshare_home_action(&player, parts[1], parts[2]);
            }
            "fav" | "favorite" => {
                if parts.len() < 2 {
                    play_error_sound(&player);
                    sender.send_error(legacy("§eИспользование: §b/home fav <название_дома>"));
                    return Ok(0);
                }
                let p_name = player.get_name();
                let res = with_state(|st| toggle_favorite_home(&mut st.store, &p_name, parts[1]));
                match res {
                    Some(true) => {
                        save_current_store();
                        play_success_sound(&player);
                        sender.send_system_message(legacy(&format!(
                            "§6§l✦ KRONOS §8» §6★ Дом §e{} §6назначен главным!",
                            parts[1]
                        )));
                    }
                    Some(false) => {
                        save_current_store();
                        play_success_sound(&player);
                        sender.send_system_message(legacy(&format!(
                            "§6§l✦ KRONOS §8» §7Отметка главного дома снята с §e{}§7.",
                            parts[1]
                        )));
                    }
                    None => {
                        play_error_sound(&player);
                        sender.send_error(legacy(&format!(
                            "§cТочка дома '§e{}§c' не найдена!",
                            parts[1]
                        )));
                    }
                }
            }
            target => {
                if let Some((owner, home_name)) = target.split_once(':') {
                    teleport_to_shared_home(&player, &server, owner, home_name);
                } else {
                    teleport_to_own_home(&player, &server, Some(target));
                }
            }
        }

        Ok(0)
    }
}

struct HomesMenuCmd;
impl CommandHandler for HomesMenuCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        let snap = build_homes_snapshot(&player);
        open_homes_menu(&player, &snap);
        Ok(0)
    }
}

struct ShareHomeCmd;
impl CommandHandler for ShareHomeCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };

        let raw = extract_args(&args);
        let parts: Vec<&str> = raw.split_whitespace().collect();

        if parts.is_empty() || parts[0].eq_ignore_ascii_case("list") {
            let p_name = player.get_name();
            let (own, shared) = with_state(|st| {
                (
                    list_player_homes(&st.store, &p_name),
                    list_shared_homes_for(&st.store, &p_name),
                )
            });
            send_shared_homes_menu(&player, &own, &shared);
            return Ok(0);
        }

        if parts[0].eq_ignore_ascii_case("unshare") || parts[0].eq_ignore_ascii_case("remove") {
            if parts.len() < 3 {
                play_error_sound(&player);
                sender.send_error(legacy(
                    "§eИспользование: §b/sharehome remove <дом> <ник_друга>",
                ));
                return Ok(0);
            }
            handle_unshare_home_action(&player, parts[1], parts[2]);
            return Ok(0);
        }

        if parts.len() < 2 {
            play_error_sound(&player);
            sender.send_error(legacy(
                "§eИспользование: §b/sharehome <дом> <ник_друга> §7или §b/sharehome list",
            ));
            return Ok(0);
        }

        handle_share_home_action(&player, &server, parts[0], parts[1]);
        Ok(0)
    }
}

struct DelHomeCmd;
impl CommandHandler for DelHomeCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };

        let raw = extract_args(&args);
        let home_name = raw.trim().split_whitespace().next().unwrap_or("");
        if home_name.is_empty() {
            play_error_sound(&player);
            sender.send_error(legacy("§eИспользование: §b/delhome <название>"));
            return Ok(0);
        }

        let p_name = player.get_name();
        let removed = with_state(|st| delete_home(&mut st.store, &p_name, home_name));

        if removed {
            save_current_store();
            play_success_sound(&player);
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS §8» §cТочка дома '§e{}§c' удалена.",
                home_name
            )));
        } else {
            play_error_sound(&player);
            sender.send_error(legacy(&format!(
                "§cТочка дома '§e{}§c' не найдена!",
                home_name
            )));
        }

        Ok(0)
    }
}

struct SpawnCmd;
impl CommandHandler for SpawnCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        run_spawn_teleport(&player, &server);
        Ok(0)
    }
}

struct SetSpawnCmd;
impl CommandHandler for SetSpawnCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        let loc = capture_player_location(&player);
        with_state(|st| {
            st.store.spawn = Some(loc.clone());
        });
        save_current_store();
        play_success_sound(&player);
        sender.send_system_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §aТочка спавна сервера обновлена: §e{} ({:.1}, {:.1}, {:.1})",
            loc.world, loc.x, loc.y, loc.z
        )));
        Ok(0)
    }
}

struct BackCmd;
impl CommandHandler for BackCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        run_back_teleport(&player, &server);
        Ok(0)
    }
}

struct RtpCmd;
impl CommandHandler for RtpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        run_rtp_teleport(&player, &server);
        Ok(0)
    }
}

struct WarpCmd;
impl CommandHandler for WarpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };

        let raw = extract_args(&args);
        let warp_name = raw.trim().split_whitespace().next().unwrap_or("");
        if warp_name.is_empty() {
            let warps = with_state(|st| list_warps(&st.store));
            send_warps_menu(&player, &warps);
            return Ok(0);
        }

        if is_combat_blocked(&player) || check_tp_cooldown(&player) {
            return Ok(0);
        }

        let (warp_opt, cfg) = with_state(|st| {
            (
                resolve_warp(&mut st.store, warp_name),
                st.config.clone(),
            )
        });

        let Some(warp) = warp_opt else {
            play_error_sound(&player);
            sender.send_error(legacy(&format!(
                "§cВарп '§e{}§c' не найден! Список: §e/warps",
                warp_name
            )));
            return Ok(0);
        };

        record_back_location(&player);
        match execute_teleport(
            &player,
            &server,
            &warp.location,
            "§b§l⚑ ВАРП",
            &format!("§e{}", warp.name),
            &cfg,
            false,
            false,
        ) {
            Ok(()) => {
                apply_tp_cooldown(&player);
                save_current_store();
                sender.send_system_message(legacy(&format!(
                    "§6§l✦ KRONOS §8» §aВы телепортированы на варп §b{}§a!",
                    warp.name
                )));
            }
            Err(err) => {
                play_error_sound(&player);
                sender.send_error(legacy(&format!("§cОшибка: {}", err)));
            }
        }

        Ok(0)
    }
}

struct WarpsListCmd;
impl CommandHandler for WarpsListCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        let warps = with_state(|st| list_warps(&st.store));
        send_warps_menu(&player, &warps);
        Ok(0)
    }
}

struct SetWarpCmd;
impl CommandHandler for SetWarpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };

        let raw = extract_args(&args);
        let mut parts = raw.split_whitespace();
        let Some(raw_name) = parts.next() else {
            play_error_sound(&player);
            sender.send_error(legacy("§eИспользование: §b/setwarp <название> [описание]"));
            return Ok(0);
        };

        let Some(warp_name) = sanitize_point_name(raw_name) else {
            play_error_sound(&player);
            sender.send_error(legacy(
                "§cНекорректное название варпа (только буквы, цифры, '_' и '-').",
            ));
            return Ok(0);
        };

        let description: String = parts.collect::<Vec<&str>>().join(" ");
        let loc = capture_player_location(&player);
        let creator = player.get_name();

        with_state(|st| {
            upsert_warp(&mut st.store, &warp_name, loc.clone(), description, creator);
        });
        save_current_store();
        play_success_sound(&player);
        sender.send_system_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §aВарп §b{} §aсоздан! §7({:.1}, {:.1}, {:.1})",
            warp_name, loc.x, loc.y, loc.z
        )));
        Ok(0)
    }
}

struct DelWarpCmd;
impl CommandHandler for DelWarpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let raw = extract_args(&args);
        let warp_name = raw.trim().split_whitespace().next().unwrap_or("");
        if warp_name.is_empty() {
            sender.send_error(legacy("§eИспользование: §b/delwarp <название>"));
            return Ok(0);
        }

        let removed = with_state(|st| delete_warp(&mut st.store, warp_name));
        if removed {
            save_current_store();
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS §8» §cВарп '§e{}§c' удалён.",
                warp_name
            )));
        } else {
            sender.send_error(legacy(&format!("§cВарп '§e{}§c' не найден!", warp_name)));
        }

        Ok(0)
    }
}

struct TpaCmd {
    bring_here: bool,
}
impl CommandHandler for TpaCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        if is_combat_blocked(&player) {
            return Ok(0);
        }

        let raw = extract_args(&args);
        let target_name = raw.trim().split_whitespace().next().unwrap_or("");
        if target_name.is_empty() {
            play_error_sound(&player);
            let cmd = if self.bring_here { "/tpahere" } else { "/tpa" };
            sender.send_error(legacy(&format!("§eИспользование: §b{} <ник_игрока>", cmd)));
            return Ok(0);
        }

        let requester_name = player.get_name();
        if requester_name.eq_ignore_ascii_case(target_name) {
            play_error_sound(&player);
            sender.send_error(legacy("§cНельзя отправить запрос самому себе!"));
            return Ok(0);
        }

        let Some(target_player) = server.get_player_by_name(target_name) else {
            play_error_sound(&player);
            sender.send_error(legacy(&format!("§cИгрок '§e{}§c' не в сети!", target_name)));
            return Ok(0);
        };

        let exact_target = target_player.get_name();
        let now = now_ms();
        let timeout_secs = with_state(|st| {
            let timeout = st.config.tpa_timeout_seconds.max(15);
            st.tpa_requests.insert(
                normalize_key(&exact_target),
                TpaRequest {
                    requester: requester_name.clone(),
                    target: exact_target.clone(),
                    bring_target_here: self.bring_here,
                    expires_at_ms: now.saturating_add(timeout * 1000),
                },
            );
            timeout
        });

        play_success_sound(&player);
        sender.send_system_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §aЗапрос телепортации отправлен игроку §e{} §7(действует {}с).",
            exact_target, timeout_secs
        )));

        play_chime_sound(&target_player);
        send_tpa_prompt(&target_player, &requester_name, self.bring_here);
        Ok(0)
    }
}

struct TpAcceptCmd;
impl CommandHandler for TpAcceptCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        if is_combat_blocked(&player) {
            return Ok(0);
        }

        let p_name = player.get_name();
        let now = now_ms();
        let (req_opt, cfg) = with_state(|st| {
            st.cleanup_expired(now);
            (st.tpa_requests.remove(&normalize_key(&p_name)), st.config.clone())
        });

        let Some(req) = req_opt else {
            play_error_sound(&player);
            sender.send_error(legacy("§cУ вас нет активных входящих запросов /tpa."));
            return Ok(0);
        };

        let Some(other_player) = server.get_player_by_name(&req.requester) else {
            play_error_sound(&player);
            sender.send_error(legacy(&format!(
                "§cИгрок §e{} §cвышел с сервера.",
                req.requester
            )));
            return Ok(0);
        };

        if is_combat_blocked(&other_player) {
            play_error_sound(&player);
            sender.send_error(legacy(&format!(
                "§cИгрок §e{} §cсейчас в бою — телепортация отменена!",
                req.requester
            )));
            return Ok(0);
        }

        let (traveler, dest_player) = if req.bring_target_here {
            (&player, &other_player)
        } else {
            (&other_player, &player)
        };

        let dest_loc = capture_player_location(dest_player);
        let dest_name = dest_player.get_name();
        record_back_location(traveler);

        match execute_teleport(
            traveler,
            &server,
            &dest_loc,
            "§6§l✦ TPA",
            &format!("§eК игроку {}", dest_name),
            &cfg,
            false,
            false,
        ) {
            Ok(()) => {
                traveler.send_system_message(
                    legacy(&format!(
                        "§6§l✦ KRONOS §8» §aВы телепортированы к игроку §e{}§a!",
                        dest_name
                    )),
                    false,
                );
                dest_player.send_system_message(
                    legacy(&format!(
                        "§6§l✦ KRONOS §8» §aИгрок §e{} §aтелепортирован к вам!",
                        traveler.get_name()
                    )),
                    false,
                );
            }
            Err(err) => {
                play_error_sound(&player);
                sender.send_error(legacy(&format!("§cОшибка телепортации: {}", err)));
            }
        }

        Ok(0)
    }
}

struct TpDenyCmd;
impl CommandHandler for TpDenyCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        let p_name = player.get_name();
        let req_opt = with_state(|st| st.tpa_requests.remove(&normalize_key(&p_name)));

        let Some(req) = req_opt else {
            play_error_sound(&player);
            sender.send_error(legacy("§cУ вас нет входящих запросов /tpa."));
            return Ok(0);
        };

        play_click_sound(&player);
        sender.send_system_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §eЗапрос от §f{} §eотклонён.",
            req.requester
        )));

        if let Some(other) = server.get_player_by_name(&req.requester) {
            play_error_sound(&other);
            other.send_system_message(
                legacy(&format!(
                    "§cИгрок §e{} §cотклонил ваш запрос на телепортацию.",
                    p_name
                )),
                false,
            );
        }

        Ok(0)
    }
}

// =============================================================================
// Registration
// =============================================================================

pub fn register_all(context: &Context) {
    let sethome = Command::new(&["sethome".into()], "Установить точку дома (/sethome [имя])")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(SetHomeCmd))
        .execute(SetHomeCmd);
    context.register_command(sethome, PERM_PLAYER);

    let home = Command::new(
        &["home".into(), "h".into()],
        "Телепорт домой, share/unshare/fav (/home [имя | владелец:дом])",
    )
    .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(HomeCmd))
    .execute(HomeCmd);
    context.register_command(home, PERM_PLAYER);

    let homes = Command::new(
        &["homes".into()],
        "Интерактивное меню ваших домов и быстрых телепортов",
    )
    .execute(HomesMenuCmd);
    context.register_command(homes, PERM_PLAYER);

    let menu = Command::new(
        &["menu".into(), "hmenu".into()],
        "Открыть интерактивный хаб KRONOS Essentials",
    )
    .execute(HomesMenuCmd);
    context.register_command(menu, PERM_PLAYER);

    let sharehome = Command::new(
        &["sharehome".into(), "sharedhomes".into()],
        "Поделиться домом с другом или открыть список общих домов",
    )
    .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(ShareHomeCmd))
    .execute(ShareHomeCmd);
    context.register_command(sharehome, PERM_PLAYER);

    let delhome = Command::new(&["delhome".into()], "Удалить точку дома (/delhome <имя>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(DelHomeCmd))
        .execute(DelHomeCmd);
    context.register_command(delhome, PERM_PLAYER);

    let spawn = Command::new(&["spawn".into()], "Телепортироваться на спавн сервера")
        .execute(SpawnCmd);
    context.register_command(spawn, PERM_PLAYER);

    let setspawn = Command::new(&["setspawn".into()], "Установить точку спавна сервера")
        .execute(SetSpawnCmd);
    context.register_command(setspawn, PERM_ADMIN);

    let back = Command::new(&["back".into()], "Вернуться на предыдущую локацию или место смерти")
        .execute(BackCmd);
    context.register_command(back, PERM_PLAYER);

    let rtp = Command::new(&["rtp".into()], "Безопасная случайная телепортация")
        .execute(RtpCmd);
    context.register_command(rtp, PERM_PLAYER);

    let warp = Command::new(&["warp".into(), "w".into()], "Телепортироваться на варп (/warp <название>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(WarpCmd))
        .execute(WarpCmd);
    context.register_command(warp, PERM_PLAYER);

    let warps = Command::new(&["warps".into()], "Интерактивный список варпов сервера")
        .execute(WarpsListCmd);
    context.register_command(warps, PERM_PLAYER);

    let setwarp = Command::new(&["setwarp".into()], "Создать точку варпа (/setwarp <имя> [описание])")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(SetWarpCmd))
        .execute(SetWarpCmd);
    context.register_command(setwarp, PERM_ADMIN);

    let delwarp = Command::new(&["delwarp".into()], "Удалить точку варпа (/delwarp <имя>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(DelWarpCmd))
        .execute(DelWarpCmd);
    context.register_command(delwarp, PERM_ADMIN);

    let tpa = Command::new(&["tpa".into()], "Запросить телепортацию к игроку (/tpa <ник>)")
        .then(
            CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
                .execute(TpaCmd { bring_here: false }),
        )
        .execute(TpaCmd { bring_here: false });
    context.register_command(tpa, PERM_PLAYER);

    let tpahere = Command::new(
        &["tpahere".into()],
        "Пригласить игрока телепортироваться к себе (/tpahere <ник>)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(TpaCmd { bring_here: true }),
    )
    .execute(TpaCmd { bring_here: true });
    context.register_command(tpahere, PERM_PLAYER);

    let tpaccept = Command::new(&["tpaccept".into()], "Принять входящий запрос /tpa")
        .execute(TpAcceptCmd);
    context.register_command(tpaccept, PERM_PLAYER);

    let tpdeny = Command::new(&["tpdeny".into()], "Отклонить входящий запрос /tpa")
        .execute(TpDenyCmd);
    context.register_command(tpdeny, PERM_PLAYER);
}
