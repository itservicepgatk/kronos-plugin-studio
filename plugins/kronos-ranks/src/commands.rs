use crate::hierarchy;
use crate::host;
use crate::menu;
use crate::models::{
    parse_duration_seconds, PlayerRankProfile, RankCategory, RanksConfig, RanksStore,
    StaffApplication,
};
use crate::tab_chat;
use std::collections::HashMap;

pub fn refresh_if_online(
    config: &RanksConfig,
    store: &RanksStore,
    online_keys: &HashMap<String, String>,
    target_name: &str,
) {
    let key = hierarchy::normalize_key(target_name);
    if let (Some(pid), Some(prof)) = (online_keys.get(&key), store.profiles.get(&key)) {
        tab_chat::refresh_player_display(config, pid, prof, online_keys.len().max(1));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_command(
    config: &RanksConfig,
    store: &mut RanksStore,
    online_keys: &HashMap<String, String>,
    dirty: &mut bool,
    player_id: &str,
    raw_cmd: &str,
) -> bool {
    let parts: Vec<&str> = raw_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }

    let cmd = parts[0].trim_start_matches('/').to_lowercase();
    let actor_is_console = player_id == "CONSOLE";
    let actor_name = store
        .id_to_name
        .get(player_id)
        .cloned()
        .unwrap_or_else(|| player_id.to_string());
    let actor_key = hierarchy::normalize_key(&actor_name);
    let now = host::now_ms();

    match cmd.as_str() {
        "donate" | "ranks" => {
            menu::open_donate_showcase(config, store, player_id);
            true
        }
        "staff" => {
            if parts.len() == 1 {
                menu::open_staff_departments(config, store, online_keys, player_id);
                return true;
            }
            match parts[1].to_lowercase().as_str() {
                "online" => {
                    host::send_chat(player_id, "§b§l✦ Персонал KRONOS онлайн по отделам:");
                    let mut found_any = false;
                    for rank in config.staff_ranks().into_iter().rev() {
                        let members: Vec<String> = store
                            .profiles
                            .values()
                            .filter(|p| {
                                p.staff_rank.as_deref() == Some(rank.id.as_str())
                                    && online_keys.contains_key(&p.player_key)
                            })
                            .map(|p| p.player_name.clone())
                            .collect();
                        if !members.is_empty() {
                            found_any = true;
                            host::send_chat(
                                player_id,
                                &format!(
                                    "  {} §8({}§8): §f{}",
                                    rank.prefix.trim(),
                                    rank.department.label_ru(),
                                    members.join(", ")
                                ),
                            );
                        }
                    }
                    if !found_any {
                        host::send_chat(player_id, "§7Сейчас нет сотрудников онлайн.");
                    }
                }
                "apply" => {
                    if parts.len() < 3 {
                        host::send_chat(
                            player_id,
                            "§dИспользование: §f/staff apply <возраст, опыт, желаемый отдел>",
                        );
                        return true;
                    }
                    let note = parts[2..].join(" ");
                    let id = store.next_app_id.max(1);
                    store.next_app_id = id + 1;
                    store.applications.push(StaffApplication {
                        id,
                        applicant_key: actor_key.clone(),
                        applicant_name: actor_name.clone(),
                        note: note.clone(),
                        created_at_ms: now,
                        status: "pending".into(),
                        reviewed_by: None,
                    });
                    *dirty = true;
                    host::send_chat(
                        player_id,
                        &format!(
                            "§a✓ Заявка #{} отправлена в §dОтдел Набора (HR)§a! Куратор рассмотрит её в §f/staff hr§a.",
                            id
                        ),
                    );
                    // Notify online HR / Management staff
                    for (okey, opid) in online_keys {
                        if let Some(prof) = store.profiles.get(okey) {
                            if hierarchy::has_permission(config, prof, "kronos.staff.hr.panel") {
                                host::send_chat(
                                    opid,
                                    &format!(
                                        "§8[§d§lHR§8] §7Новая заявка #{} от §f{}§7: §e{}",
                                        id, actor_name, note
                                    ),
                                );
                            }
                        }
                    }
                }
                "hr" | "recruit" => {
                    let allowed = actor_is_console
                        || store
                            .profiles
                            .get(&actor_key)
                            .map(|p| hierarchy::has_permission(config, p, "kronos.staff.hr.panel"))
                            .unwrap_or(false);
                    if !allowed {
                        host::send_chat(player_id, "§c✗ HR-панель доступна Куратору Набора, Гл.Куратору и Владельцу.");
                    } else {
                        menu::open_hr_panel(store, player_id);
                    }
                }
                "audit" => {
                    let allowed = actor_is_console
                        || store
                            .profiles
                            .get(&actor_key)
                            .map(|p| hierarchy::has_permission(config, p, "kronos.staff.hr.audit"))
                            .unwrap_or(false);
                    if !allowed {
                        host::send_chat(player_id, "§c✗ Аудит доступен с должности §dКуратор Набора§c.");
                    } else {
                        menu::open_audit_menu(store, player_id);
                    }
                }
                _ => menu::open_staff_departments(config, store, online_keys, player_id),
            }
            true
        }
        "rank" | "setrank" => {
            // Support direct `/setrank <player> <rank> [duration]` alias
            let args: Vec<&str> = if cmd == "setrank" {
                let mut v = vec!["set"];
                v.extend_from_slice(&parts[1..]);
                v
            } else {
                parts[1..].to_vec()
            };

            if args.is_empty() {
                menu::open_player_profile(config, store, player_id, &actor_name);
                return true;
            }

            match args[0].to_lowercase().as_str() {
                "info" | "profile" => {
                    let target = if args.len() >= 2 { args[1] } else { &actor_name };
                    menu::open_player_profile(config, store, player_id, target);
                }
                "list" => {
                    host::send_chat(player_id, "§6§l✦ Донат-ветка (10 рангов):");
                    let donate_line: Vec<String> = config
                        .donate_ranks()
                        .into_iter()
                        .map(|r| format!("{}§8({})", r.prefix.trim(), r.id))
                        .collect();
                    host::send_chat(player_id, &donate_line.join(" §7→ "));

                    host::send_chat(player_id, "§b§l✦ Стафф-ветка (9 должностей по отделам):");
                    for r in config.staff_ranks() {
                        host::send_chat(
                            player_id,
                            &format!(
                                "  §8[§f{}§8] {} §7({}) — {}",
                                r.weight,
                                r.prefix.trim(),
                                r.id,
                                r.department.label_ru()
                            ),
                        );
                    }
                }
                "set" | "give" => {
                    if args.len() < 3 {
                        host::send_chat(
                            player_id,
                            "§eИспользование: §f/rank set <игрок> <ранг> [срок: 30d|perm]",
                        );
                        return true;
                    }
                    let dur = if args.len() >= 4 {
                        match parse_duration_seconds(args[3]) {
                            Ok(d) => d,
                            Err(_) => {
                                host::send_chat(player_id, "§c✗ Неверный срок (пример: 7d, 30d, perm).");
                                return true;
                            }
                        }
                    } else {
                        None
                    };

                    match hierarchy::assign_rank(
                        config,
                        store,
                        dirty,
                        if actor_is_console { None } else { Some(&actor_key) },
                        &actor_name,
                        actor_is_console,
                        args[1],
                        args[2],
                        dur,
                        now,
                    ) {
                        Ok((_, def)) => {
                            refresh_if_online(config, store, online_keys, args[1]);
                            let track_ru = if def.category == RankCategory::Staff {
                                format!("Стафф ({})", def.department.label_ru())
                            } else {
                                "Донат".into()
                            };
                            host::send_chat(
                                player_id,
                                &format!(
                                    "§a✓ Игроку §f{} §aвыдан ранг {} §8[{}§8]!",
                                    args[1],
                                    def.prefix.trim(),
                                    track_ru
                                ),
                            );
                            if let Some(tpid) = online_keys.get(&hierarchy::normalize_key(args[1])) {
                                host::send_title(
                                    tpid,
                                    &format!("§6✦ {}", def.prefix.trim()),
                                    &format!("§fВам выдан статус {}", def.display_name),
                                    10,
                                    60,
                                    15,
                                );
                                host::play_sound(tpid, "minecraft:ui.toast.challenge_complete", 1.0, 1.1);
                            }
                        }
                        Err(e) => host::send_chat(player_id, &e),
                    }
                }
                "remove" | "demote" => {
                    if args.len() < 2 {
                        host::send_chat(player_id, "§eИспользование: §f/rank remove <игрок> [staff|donate]");
                        return true;
                    }
                    let track = if args.len() >= 3 { args[2] } else { "staff" };
                    match hierarchy::remove_rank_track(
                        config,
                        store,
                        dirty,
                        if actor_is_console { None } else { Some(&actor_key) },
                        &actor_name,
                        actor_is_console,
                        args[1],
                        track,
                        now,
                    ) {
                        Ok(_) => {
                            refresh_if_online(config, store, online_keys, args[1]);
                            host::send_chat(
                                player_id,
                                &format!("§a✓ Ветка §e{} §aсброшена у игрока §f{}§a.", track, args[1]),
                            );
                        }
                        Err(e) => host::send_chat(player_id, &e),
                    }
                }
                "perm" => {
                    let allowed = actor_is_console
                        || store
                            .profiles
                            .get(&actor_key)
                            .map(|p| hierarchy::has_permission(config, p, "kronos.ranks.perm.manage"))
                            .unwrap_or(false);
                    if !allowed {
                        host::send_chat(player_id, "§c✗ Управление правами доступно с §bТех.Админ§c.");
                        return true;
                    }
                    if args.len() < 4 {
                        host::send_chat(
                            player_id,
                            "§eИспользование: §f/rank perm <игрок> <add|remove|check> <право>",
                        );
                        return true;
                    }
                    let tkey = hierarchy::normalize_key(args[1]);
                    let node = args[3];
                    match args[2].to_lowercase().as_str() {
                        "add" => {
                            let prof = store
                                .profiles
                                .entry(tkey)
                                .or_insert_with(|| PlayerRankProfile::new(args[1], now));
                            if !prof.extra_permissions.iter().any(|p| p == node) {
                                prof.extra_permissions.push(node.to_string());
                                *dirty = true;
                            }
                            host::send_chat(player_id, &format!("§a✓ Право §e{} §aдобавлено §f{}.", node, args[1]));
                        }
                        "remove" => {
                            if let Some(prof) = store.profiles.get_mut(&tkey) {
                                prof.extra_permissions.retain(|p| p != node);
                                *dirty = true;
                            }
                            host::send_chat(player_id, &format!("§eПраво §f{} §eудалено у §f{}.", node, args[1]));
                        }
                        "check" => {
                            let prof = store
                                .profiles
                                .get(&tkey)
                                .cloned()
                                .unwrap_or_else(|| PlayerRankProfile::new(args[1], now));
                            let has = hierarchy::has_permission(config, &prof, node);
                            host::send_chat(
                                player_id,
                                &format!(
                                    "§7Право §e{} §7у §f{}: {}",
                                    node,
                                    args[1],
                                    if has { "§aЕСТЬ" } else { "§cНЕТ" }
                                ),
                            );
                        }
                        _ => host::send_chat(player_id, "§cДействие: add | remove | check"),
                    }
                }
                "suffix" => {
                    let prof = store
                        .profiles
                        .entry(actor_key.clone())
                        .or_insert_with(|| PlayerRankProfile::new(&actor_name, now));
                    let perks = hierarchy::effective_perks(config, prof);
                    if !perks.custom_suffix {
                        host::send_chat(player_id, "§c✗ Кастомный суффикс доступен с доната §d§lEMPEROR§c!");
                        return true;
                    }
                    if args.len() < 2 || args[1].eq_ignore_ascii_case("clear") {
                        prof.custom_suffix = None;
                        *dirty = true;
                        refresh_if_online(config, store, online_keys, &actor_name);
                        host::send_chat(player_id, "§7Суффикс сброшен.");
                    } else {
                        let raw = args[1..].join(" ");
                        if raw.len() > 24 {
                            host::send_chat(player_id, "§c✗ Максимальная длина суффикса — 24 символа.");
                            return true;
                        }
                        prof.custom_suffix = Some(tab_chat::translate_ampersand_colors(&raw, perks.rgb_chat));
                        *dirty = true;
                        refresh_if_online(config, store, online_keys, &actor_name);
                        host::send_chat(player_id, "§a✓ Новый суффикс установлен!");
                    }
                }
                _ => menu::open_donate_showcase(config, store, player_id),
            }
            true
        }
        _ => false,
    }
}
