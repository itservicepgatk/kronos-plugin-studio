use crate::host;
use crate::inspector;
use crate::menu;
use crate::models::{
    format_duration_ru, parse_duration_seconds, BlockLogEntry, GuardConfig, GuardStore,
    PunishmentKind, ReportEntry, RuntimeSession,
};
use crate::punishments::{
    active_warn_count, check_hierarchy, issue_punishment, normalize_key, notify_staff,
    revoke_punishment,
};
use crate::storage;
use std::collections::VecDeque;

pub fn toggle_vanish(
    config: &GuardConfig,
    store: &GuardStore,
    session: &mut RuntimeSession,
    player_id: &str,
) {
    let rank = storage::effective_rank(store, player_id);
    let perms = config.perms_for(&rank);
    if !perms.can_vanish {
        host::send_chat(player_id, "§c✗ Ваниш доступен с ранга §9Модер§c.");
        return;
    }

    let name = store
        .known_names
        .get(player_id)
        .cloned()
        .unwrap_or_else(|| player_id.to_string());

    if session.vanished.remove(player_id) {
        host::set_vanished(player_id, false, 0);
        host::send_chat(player_id, "§7Ваниш §cвыключен§7 — вы снова видны игрокам.");
        host::play_sound(player_id, "minecraft:entity.enderman.teleport", 0.7, 0.8);
        if config.vanish.fake_leave_join_messages {
            host::broadcast_chat(&format!("§e{} зашёл на сервер", name));
        }
        notify_staff(
            config,
            session,
            store,
            &format!("§8[§bVanish§8] §f{} §7вышел из ваниша.", name),
        );
    } else {
        session.vanished.insert(player_id.to_string());
        host::set_vanished(player_id, true, perms.weight);
        host::send_chat(
            player_id,
            "§b👻 Ваниш §aВКЛЮЧЁН§b! Вы скрыты от игроков, сундуки открываются бесшумно.",
        );
        host::play_sound(player_id, "minecraft:entity.enderman.teleport", 0.7, 1.4);
        if config.vanish.fake_leave_join_messages {
            host::broadcast_chat(&format!("§e{} вышел с сервера", name));
        }
        notify_staff(
            config,
            session,
            store,
            &format!("§8[§bVanish§8] §f{} §bвошёл в ваниш.", name),
        );
    }
}

pub fn toggle_inspector(
    config: &GuardConfig,
    store: &GuardStore,
    session: &mut RuntimeSession,
    player_id: &str,
) {
    let rank = storage::effective_rank(store, player_id);
    if !config.perms_for(&rank).can_inspect {
        host::send_chat(player_id, "§c✗ Инспектор блоков доступен с ранга §eСтажёр§c.");
        return;
    }

    if session.inspectors.remove(player_id) {
        host::send_chat(player_id, "§7Режим инспектора блоков §cвыключен§7.");
        host::play_sound(player_id, "minecraft:ui.button.click", 0.7, 0.8);
    } else {
        session.inspectors.insert(player_id.to_string());
        host::give_item(
            player_id,
            &config.inspector_item,
            "§6§lKRONOS Inspector",
            "§7ЛКМ/ПКМ по блоку или сундуку\n§7чтобы увидеть кто его трогал",
        );
        host::send_chat(
            player_id,
            "§6✦ Режим инспектора §aВКЛЮЧЁН§6! Кликните по блоку или сундуку, чтобы увидеть журнал.",
        );
        host::play_sound(player_id, "minecraft:entity.experience_orb.pickup", 0.8, 1.3);
    }
}

pub fn toggle_freeze(
    config: &GuardConfig,
    store: &GuardStore,
    session: &mut RuntimeSession,
    staff_id: &str,
    target_name: &str,
) {
    let staff_rank = storage::effective_rank(store, staff_id);
    if !config.perms_for(&staff_rank).can_freeze {
        host::send_chat(staff_id, "§c✗ Заморозка доступна с ранга §9Модер§c.");
        return;
    }
    if let Err(err) = check_hierarchy(config, store, staff_id, &staff_rank, target_name) {
        host::send_chat(staff_id, &err);
        return;
    }

    let key = normalize_key(target_name);
    let staff_name = store
        .known_names
        .get(staff_id)
        .cloned()
        .unwrap_or_else(|| staff_id.to_string());

    if session.frozen.remove(&key).is_some() {
        if let Some(target_pid) = session.online_ids.get(&key) {
            host::set_frozen(target_pid, false);
            host::send_title(target_pid, "§aПРОВЕРКА ПРОЙДЕНА", "§7Вы разморожены", 5, 40, 10);
        }
        host::send_chat(staff_id, &format!("§a✓ Игрок §f{} §aразморожен.", target_name));
    } else {
        session.frozen.insert(key.clone(), staff_name.clone());
        if let Some(target_pid) = session.online_ids.get(&key) {
            host::set_frozen(target_pid, true);
            host::send_title(
                target_pid,
                "§b§lВЫ ЗАМОРОЖЕНЫ!",
                "§cНе выходите с сервера — авто-бан 7д!",
                5,
                120,
                20,
            );
            host::send_chat(
                target_pid,
                &format!(
                    "§b❄ Модератор §e{} §bвызвал вас на проверку! Не выходите с сервера (авто-бан §c{}§b).",
                    staff_name, config.freeze_logout_ban_duration
                ),
            );
            host::play_sound(target_pid, "minecraft:block.glass.break", 1.0, 0.7);
        }
        host::send_chat(
            staff_id,
            &format!("§b❄ Игрок §f{} §bзаморожен. При выходе он получит авто-бан.", target_name),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_command(
    store: &mut GuardStore,
    block_log: &mut VecDeque<BlockLogEntry>,
    config: &GuardConfig,
    session: &mut RuntimeSession,
    dirty_store: &mut bool,
    dirty_log: &mut bool,
    player_id: &str,
    raw_cmd: &str,
) -> bool {
    let parts: Vec<&str> = raw_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }

    let cmd = parts[0].trim_start_matches('/').to_lowercase();
    let staff_rank = storage::effective_rank(store, player_id);
    let staff_name = store
        .known_names
        .get(player_id)
        .cloned()
        .unwrap_or_else(|| player_id.to_string());
    let now = host::now_ms();

    match cmd.as_str() {
        "ban" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /ban <игрок> [причина]");
                return true;
            }
            let reason = if parts.len() > 2 { parts[2..].join(" ") } else { String::new() };
            if let Err(e) = issue_punishment(
                store,
                config,
                session,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                PunishmentKind::Ban,
                parts[1],
                None,
                &reason,
                now,
            ) {
                host::send_chat(player_id, &e);
            }
            true
        }
        "tempban" => {
            if parts.len() < 3 {
                host::send_chat(player_id, "§eИспользование: /tempban <игрок> <срок: 10m|2h|7d> [причина]");
                return true;
            }
            match parse_duration_seconds(parts[2]) {
                Ok(Some(secs)) => {
                    let reason = if parts.len() > 3 { parts[3..].join(" ") } else { String::new() };
                    if let Err(e) = issue_punishment(
                        store,
                        config,
                        session,
                        dirty_store,
                        player_id,
                        &staff_name,
                        &staff_rank,
                        PunishmentKind::TempBan,
                        parts[1],
                        Some(secs),
                        &reason,
                        now,
                    ) {
                        host::send_chat(player_id, &e);
                    }
                }
                _ => host::send_chat(player_id, "§c✗ Неверный формат срока (пример: 30m, 12h, 7d)."),
            }
            true
        }
        "ipban" | "banip" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /ipban <игрок> [причина]");
                return true;
            }
            let reason = if parts.len() > 2 { parts[2..].join(" ") } else { String::new() };
            if let Err(e) = issue_punishment(
                store,
                config,
                session,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                PunishmentKind::IpBan,
                parts[1],
                None,
                &reason,
                now,
            ) {
                host::send_chat(player_id, &e);
            }
            true
        }
        "unban" | "pardon" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /unban <игрок>");
                return true;
            }
            match revoke_punishment(
                store,
                config,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                parts[1],
                Some(PunishmentKind::Ban),
                now,
            ) {
                Ok(0) => host::send_chat(player_id, &format!("§7У игрока §f{} §7нет активного бана.", parts[1])),
                Ok(_) => host::broadcast_chat(&format!(
                    "§8[§aGUARD§8] §f{} §7разбанен модератором §e{}",
                    parts[1], staff_name
                )),
                Err(e) => host::send_chat(player_id, &e),
            }
            true
        }
        "mute" | "tempmute" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /mute <игрок> [срок: 15m|2h|1d] [причина]");
                return true;
            }
            let (dur, reason) = if parts.len() >= 3 {
                match parse_duration_seconds(parts[2]) {
                    Ok(d) => (d, if parts.len() > 3 { parts[3..].join(" ") } else { String::new() }),
                    Err(_) => (Some(1800), parts[2..].join(" ")),
                }
            } else {
                (Some(1800), String::new())
            };
            if let Err(e) = issue_punishment(
                store,
                config,
                session,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                PunishmentKind::Mute,
                parts[1],
                dur,
                &reason,
                now,
            ) {
                host::send_chat(player_id, &e);
            }
            true
        }
        "unmute" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /unmute <игрок>");
                return true;
            }
            match revoke_punishment(
                store,
                config,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                parts[1],
                Some(PunishmentKind::Mute),
                now,
            ) {
                Ok(0) => host::send_chat(player_id, &format!("§7У игрока §f{} §7нет активного мута.", parts[1])),
                Ok(_) => host::send_chat(player_id, &format!("§a✓ Мут снят с игрока §f{}§a.", parts[1])),
                Err(e) => host::send_chat(player_id, &e),
            }
            true
        }
        "warn" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /warn <игрок> [причина]");
                return true;
            }
            let reason = if parts.len() > 2 { parts[2..].join(" ") } else { String::new() };
            if let Err(e) = issue_punishment(
                store,
                config,
                session,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                PunishmentKind::Warn,
                parts[1],
                None,
                &reason,
                now,
            ) {
                host::send_chat(player_id, &e);
            }
            true
        }
        "unwarn" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /unwarn <игрок>");
                return true;
            }
            match revoke_punishment(
                store,
                config,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                parts[1],
                Some(PunishmentKind::Warn),
                now,
            ) {
                Ok(cnt) => host::send_chat(
                    player_id,
                    &format!("§a✓ Снято предупреждений с §f{}§a: §e{}", parts[1], cnt),
                ),
                Err(e) => host::send_chat(player_id, &e),
            }
            true
        }
        "warns" => {
            let target = if parts.len() >= 2 { parts[1] } else { &staff_name };
            let cnt = active_warn_count(store, target, now);
            host::send_chat(
                player_id,
                &format!("§eАктивных предупреждений у §f{}§e: §c{}", target, cnt),
            );
            true
        }
        "kick" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /kick <игрок> [причина]");
                return true;
            }
            let reason = if parts.len() > 2 { parts[2..].join(" ") } else { String::new() };
            if let Err(e) = issue_punishment(
                store,
                config,
                session,
                dirty_store,
                player_id,
                &staff_name,
                &staff_rank,
                PunishmentKind::Kick,
                parts[1],
                None,
                &reason,
                now,
            ) {
                host::send_chat(player_id, &e);
            }
            true
        }
        "vanish" | "v" => {
            toggle_vanish(config, store, session, player_id);
            true
        }
        "freeze" | "unfreeze" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /freeze <игрок>");
                return true;
            }
            toggle_freeze(config, store, session, player_id, parts[1]);
            true
        }
        "history" | "check" => {
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /history <игрок>");
                return true;
            }
            menu::open_history_menu(store, player_id, parts[1]);
            true
        }
        "sc" | "staffchat" => {
            if !config.perms_for(&staff_rank).can_staff_chat {
                host::send_chat(player_id, "§c✗ Чат персонала недоступен для вашего ранга.");
                return true;
            }
            if parts.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /sc <сообщение>");
                return true;
            }
            let prefix = config.perms_for(&staff_rank).prefix;
            notify_staff(
                config,
                session,
                store,
                &format!("§8[§bSC§8] {} §f{}§8: §b{}", prefix, staff_name, parts[1..].join(" ")),
            );
            true
        }
        "report" => {
            if parts.len() < 3 {
                host::send_chat(player_id, "§eИспользование: /report <игрок> <причина>");
                return true;
            }
            let cd_ms = config.report_cooldown_seconds * 1000;
            if let Some(&last) = session.report_cooldowns.get(player_id) {
                if now.saturating_sub(last) < cd_ms {
                    let wait = (cd_ms - (now - last)) / 1000;
                    host::send_chat(player_id, &format!("§cПодождите §e{}с §cперед новой жалобой.", wait.max(1)));
                    return true;
                }
            }
            session.report_cooldowns.insert(player_id.to_string(), now);
            let pos = host::get_player_pos(player_id);
            let rep_id = store.next_report_id.max(1);
            store.next_report_id = rep_id + 1;
            let reason = parts[2..].join(" ");
            store.reports.push(ReportEntry {
                id: rep_id,
                reporter_name: staff_name.clone(),
                target_name: parts[1].to_string(),
                reason: reason.clone(),
                world: pos.world,
                x: pos.x,
                y: pos.y,
                z: pos.z,
                created_at_ms: now,
                resolved: false,
                resolved_by: None,
            });
            *dirty_store = true;
            host::send_chat(player_id, "§a✓ Жалоба отправлена модераторам! Спасибо за помощь.");
            notify_staff(
                config,
                session,
                store,
                &format!(
                    "§8[§c§lREPORT #{}§8] §e{} §7→ §c{}§7: §f{}",
                    rep_id, staff_name, parts[1], reason
                ),
            );
            true
        }
        "guard" | "co" | "mod" => {
            handle_guard_subcommand(
                store,
                block_log,
                config,
                session,
                dirty_log,
                player_id,
                &staff_name,
                &staff_rank,
                &parts[1..],
                now,
            );
            true
        }
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_guard_subcommand(
    store: &mut GuardStore,
    block_log: &mut VecDeque<BlockLogEntry>,
    config: &GuardConfig,
    session: &mut RuntimeSession,
    dirty_log: &mut bool,
    player_id: &str,
    staff_name: &str,
    staff_rank: &str,
    args: &[&str],
    now_ms: u64,
) {
    let perms = config.perms_for(staff_rank);
    if !perms.can_inspect && !perms.can_warn {
        host::send_chat(player_id, "§c✗ Панель KRONOS Guard доступна только персоналу.");
        return;
    }

    if args.is_empty() || args[0].eq_ignore_ascii_case("menu") {
        menu::open_guard_hub(store, block_log, config, session, player_id);
        return;
    }

    match args[0].to_lowercase().as_str() {
        "i" | "inspect" => toggle_inspector(config, store, session, player_id),
        "player" | "p" => {
            if args.len() < 2 {
                host::send_chat(player_id, "§eИспользование: /guard player <игрок>");
                return;
            }
            menu::open_player_panel(store, session, player_id, args[1]);
        }
        "reports" => menu::open_reports_menu(store, player_id),
        "l" | "lookup" => {
            if !perms.can_inspect {
                host::send_chat(player_id, "§c✗ Нет прав на просмотр журнала блоков.");
                return;
            }
            let (radius, since_sec, actor) = parse_query_params(&args[1..], config, now_ms);
            let pos = host::get_player_pos(player_id);
            let entries = inspector::lookup_radius(
                block_log,
                &pos.world,
                pos.x,
                pos.y,
                pos.z,
                radius,
                since_sec,
                actor.as_deref(),
                config.block_log.page_size.max(8),
            );
            host::send_chat(
                player_id,
                &format!(
                    "§8§m---§r §6§lKRONOS Lookup §8(r:{} найдено: {}) §8§m---",
                    radius,
                    entries.len()
                ),
            );
            if entries.is_empty() {
                host::send_chat(player_id, "§7Событий по заданному фильтру не найдено.");
                return;
            }
            let now_sec = now_ms / 1000;
            for e in entries {
                host::send_chat(player_id, &inspector::format_entry_line(e, now_sec));
            }
        }
        "rb" | "rollback" => {
            if !perms.can_rollback {
                host::send_chat(player_id, "§c✗ Откат грифа доступен с ранга §3Ст.Модер§c.");
                return;
            }
            let (radius, since_sec, actor) = parse_query_params(&args[1..], config, now_ms);
            let pos = host::get_player_pos(player_id);
            match inspector::rollback_grief(
                block_log,
                store,
                config,
                session,
                dirty_log,
                player_id,
                staff_name,
                &pos.world,
                pos.x,
                pos.y,
                pos.z,
                radius,
                since_sec,
                actor.as_deref(),
                now_ms,
            ) {
                Ok(cnt) => host::send_chat(
                    player_id,
                    &format!(
                        "§a✓ Откат выполнен! Восстановлено блоков: §e{} §8(отмена: /guard undo)",
                        cnt
                    ),
                ),
                Err(e) => host::send_chat(player_id, &e),
            }
        }
        "undo" => {
            if !perms.can_rollback {
                host::send_chat(player_id, "§c✗ Отмена отката доступна с ранга §3Ст.Модер§c.");
                return;
            }
            match inspector::undo_last_rollback(session, player_id) {
                Ok(cnt) => host::send_chat(
                    player_id,
                    &format!("§e↺ Последний откат отменён (возвращено блоков: §f{}§e).", cnt),
                ),
                Err(e) => host::send_chat(player_id, &e),
            }
        }
        _ => {
            host::send_chat(player_id, "§6§lKRONOS Guard — команды:");
            host::send_chat(player_id, "§e/guard §7— открыть GUI-панель модератора");
            host::send_chat(player_id, "§e/co i §7— включить инспектор блоков и сундуков");
            host::send_chat(player_id, "§e/co l r:10 t:1h u:<ник> §7— поиск логов в радиусе");
            host::send_chat(player_id, "§e/co rb r:15 t:30m u:<ник> §7— откатить гриф");
            host::send_chat(player_id, "§e/ban /tempban /mute /warn /kick /vanish /freeze");
            host::send_chat(
                player_id,
                &format!(
                    "§8Лимит отката: r:{} | Хранение лога: {}д",
                    config.block_log.max_rollback_radius,
                    format_duration_ru(config.block_log.retention_days * 86_400)
                ),
            );
        }
    }
}

fn parse_query_params(args: &[&str], config: &GuardConfig, now_ms: u64) -> (i32, u64, Option<String>) {
    let mut radius = config.block_log.default_lookup_radius;
    let mut window_secs: u64 = 3600; // default 1h
    let mut actor: Option<String> = None;

    for arg in args {
        if let Some(r) = arg.strip_prefix("r:") {
            if let Ok(parsed) = r.parse::<i32>() {
                radius = parsed.clamp(1, config.block_log.max_lookup_radius);
            }
        } else if let Some(t) = arg.strip_prefix("t:") {
            if let Ok(Some(secs)) = parse_duration_seconds(t) {
                window_secs = secs;
            }
        } else if let Some(u) = arg.strip_prefix("u:") {
            if !u.is_empty() {
                actor = Some(u.to_string());
            }
        }
    }

    let since_sec = (now_ms / 1000).saturating_sub(window_secs);
    (radius, since_sec, actor)
}
