use crate::host;
use crate::models::{
    format_duration_ru, parse_duration_seconds, GuardConfig, GuardStore, PunishmentKind,
    PunishmentRecord, RuntimeSession,
};

pub fn normalize_key(name: &str) -> String {
    name.trim().to_lowercase()
}

pub fn notify_staff(config: &GuardConfig, session: &RuntimeSession, store: &GuardStore, msg: &str) {
    for (lower_name, pid) in &session.online_ids {
        let rank = store
            .known_ranks
            .get(lower_name)
            .cloned()
            .unwrap_or_else(|| host::get_player_rank(pid));
        if config.perms_for(&rank).can_staff_chat {
            host::send_chat(pid, msg);
        }
    }
}

pub fn check_hierarchy(
    config: &GuardConfig,
    store: &GuardStore,
    staff_id: &str,
    staff_rank: &str,
    target_name: &str,
) -> Result<(), String> {
    if staff_id == "CONSOLE" || staff_rank.eq_ignore_ascii_case("owner") {
        return Ok(());
    }
    let staff_weight = config.perms_for(staff_rank).weight;
    let target_key = normalize_key(target_name);
    let target_rank = store
        .known_ranks
        .get(&target_key)
        .map(|s| s.as_str())
        .unwrap_or("player");
    let target_weight = config.perms_for(target_rank).weight;

    if target_weight >= staff_weight {
        return Err(format!(
            "§c✗ Нельзя наказать §f{} §c— его ранг (§e{}§c) равен или выше вашего.",
            target_name, target_rank
        ));
    }
    Ok(())
}

pub fn active_ban<'a>(
    store: &'a GuardStore,
    player_name: &str,
    ip: Option<&str>,
    now_ms: u64,
) -> Option<&'a PunishmentRecord> {
    let key = normalize_key(player_name);
    store.punishments.iter().rev().find(|p| {
        if !p.kind.is_ban() || !p.is_active(now_ms) {
            return false;
        }
        if p.target_key == key {
            return true;
        }
        if p.kind == PunishmentKind::IpBan {
            if let (Some(ban_ip), Some(check_ip)) = (p.target_ip.as_deref(), ip) {
                return ban_ip == check_ip;
            }
        }
        false
    })
}

pub fn active_mute<'a>(
    store: &'a GuardStore,
    player_name: &str,
    now_ms: u64,
) -> Option<&'a PunishmentRecord> {
    let key = normalize_key(player_name);
    store
        .punishments
        .iter()
        .rev()
        .find(|p| p.kind == PunishmentKind::Mute && p.target_key == key && p.is_active(now_ms))
}

pub fn active_warn_count(store: &GuardStore, player_name: &str, now_ms: u64) -> u32 {
    let key = normalize_key(player_name);
    store
        .punishments
        .iter()
        .filter(|p| p.kind == PunishmentKind::Warn && p.target_key == key && p.is_active(now_ms))
        .count() as u32
}

pub fn format_ban_screen(config: &GuardConfig, rec: &PunishmentRecord, now_ms: u64) -> String {
    let time_line = match rec.remaining_ms(now_ms) {
        None => "§4Навсегда".to_string(),
        Some(ms) => format!("§e{}", format_duration_ru(ms / 1000)),
    };
    format!(
        "§4§l✖ {} — ДОСТУП ЗАБЛОКИРОВАН ✖\n\n§7Тип: {}\n§7Причина: §f{}\n§7Модератор: §e{}\n§7Осталось: {}\n§7ID наказания: §8#{}\n\n§7Обжаловать: §b{}",
        config.server_name,
        rec.kind.label_ru(),
        rec.reason,
        rec.staff_name,
        time_line,
        rec.id,
        config.appeal_url
    )
}

#[allow(clippy::too_many_arguments)]
pub fn issue_punishment(
    store: &mut GuardStore,
    config: &GuardConfig,
    session: &RuntimeSession,
    dirty: &mut bool,
    staff_id: &str,
    staff_name: &str,
    staff_rank: &str,
    kind: PunishmentKind,
    target_name: &str,
    duration_secs: Option<u64>,
    reason: &str,
    now_ms: u64,
) -> Result<PunishmentRecord, String> {
    check_hierarchy(config, store, staff_id, staff_rank, target_name)?;

    let perms = config.perms_for(staff_rank);
    if staff_id != "CONSOLE" {
        match kind {
            PunishmentKind::Warn => {
                if !perms.can_warn {
                    return Err("§c✗ У вашего ранга нет прав на /warn.".into());
                }
            }
            PunishmentKind::Kick => {
                if !perms.can_kick {
                    return Err("§c✗ У вашего ранга нет прав на /kick.".into());
                }
            }
            PunishmentKind::Mute => {
                if !perms.can_mute {
                    return Err("§c✗ У вашего ранга нет прав на /mute.".into());
                }
                if perms.max_mute_seconds > 0 {
                    match duration_secs {
                        None => {
                            return Err(format!(
                                "§c✗ Ваш лимит мута: §e{}§c.",
                                format_duration_ru(perms.max_mute_seconds)
                            ))
                        }
                        Some(s) if s > perms.max_mute_seconds => {
                            return Err(format!(
                                "§c✗ Превышен лимит мута! Максимум: §e{}§c.",
                                format_duration_ru(perms.max_mute_seconds)
                            ))
                        }
                        _ => {}
                    }
                }
            }
            PunishmentKind::TempBan => {
                if !perms.can_tempban {
                    return Err("§c✗ У вашего ранга нет прав на /tempban.".into());
                }
                if perms.max_ban_seconds > 0 {
                    if let Some(s) = duration_secs {
                        if s > perms.max_ban_seconds {
                            return Err(format!(
                                "§c✗ Превышен лимит tempban! Максимум: §e{}§c.",
                                format_duration_ru(perms.max_ban_seconds)
                            ));
                        }
                    }
                }
            }
            PunishmentKind::Ban => {
                if !perms.can_permban {
                    return Err("§c✗ Перманентный бан доступен с ранга §3Ст.Модер§c.".into());
                }
            }
            PunishmentKind::IpBan => {
                if !perms.can_ipban {
                    return Err("§c✗ IP-бан доступен с ранга §cАдмин§c.".into());
                }
            }
        }
    }

    let target_key = normalize_key(target_name);
    let clean_reason = if reason.trim().is_empty() {
        "Нарушение правил сервера".to_string()
    } else {
        reason.trim().to_string()
    };

    let target_ip = store.known_ips.get(&target_key).cloned();
    let id = store.next_punishment_id.max(1);
    store.next_punishment_id = id + 1;

    let expires_at_ms = duration_secs.map(|s| now_ms.saturating_add(s.saturating_mul(1000)));
    let record = PunishmentRecord {
        id,
        kind,
        target_key: target_key.clone(),
        target_name: target_name.to_string(),
        target_ip,
        staff_id: staff_id.to_string(),
        staff_name: staff_name.to_string(),
        reason: clean_reason.clone(),
        created_at_ms: now_ms,
        expires_at_ms,
        revoked_by: None,
        revoked_at_ms: None,
    };

    store.punishments.push(record.clone());
    *dirty = true;

    let dur_str = match duration_secs {
        None => "навсегда".to_string(),
        Some(s) => format_duration_ru(s),
    };

    // Broadcast concise moderation banner
    host::broadcast_chat(&format!(
        "§8[§c§lGUARD§8] {} §f{} §7← §e{} §8(§7{}§8) §8| §f{}",
        kind.label_ru(),
        target_name,
        staff_name,
        dur_str,
        clean_reason
    ));

    // Apply live effect if target is online
    if let Some(target_pid) = session.online_ids.get(&target_key) {
        match kind {
            PunishmentKind::Ban | PunishmentKind::TempBan | PunishmentKind::IpBan => {
                let screen = format_ban_screen(config, &record, now_ms);
                host::kick_player(target_pid, &screen);
            }
            PunishmentKind::Kick => {
                host::kick_player(
                    target_pid,
                    &format!(
                        "§c§l{} — ВЫ БЫЛИ КИКНУТЫ\n§7Причина: §f{}\n§7Модератор: §e{}",
                        config.server_name, clean_reason, staff_name
                    ),
                );
            }
            PunishmentKind::Mute => {
                host::send_title(
                    target_pid,
                    "§6§lВЫ ПОЛУЧИЛИ МУТ",
                    &format!("§7Срок: §e{} §8| §f{}", dur_str, clean_reason),
                    5,
                    60,
                    15,
                );
                host::play_sound(target_pid, "minecraft:entity.villager.no", 1.0, 0.8);
            }
            PunishmentKind::Warn => {
                let warns = active_warn_count(store, target_name, now_ms);
                host::send_title(
                    target_pid,
                    &format!("§e§lПРЕДУПРЕЖДЕНИЕ #{}", warns),
                    &format!("§f{}", clean_reason),
                    5,
                    60,
                    15,
                );
                host::play_sound(target_pid, "minecraft:block.anvil.land", 0.8, 1.2);
            }
        }
    }

    // Check automatic warning escalation ladder
    if kind == PunishmentKind::Warn {
        let warns = active_warn_count(store, target_name, now_ms);
        if let Some(rule) = config
            .warn_escalation
            .iter()
            .find(|r| r.warn_count == warns)
            .cloned()
        {
            let esc_secs = parse_duration_seconds(&rule.duration).unwrap_or(Some(1800));
            let esc_kind = match rule.action.as_str() {
                "mute" => PunishmentKind::Mute,
                "ban" => PunishmentKind::Ban,
                _ => PunishmentKind::TempBan,
            };
            let _ = issue_punishment(
                store,
                config,
                session,
                dirty,
                "CONSOLE",
                "KRONOS AutoGuard",
                "owner",
                esc_kind,
                target_name,
                esc_secs,
                &rule.reason,
                now_ms,
            );
        }
    }

    Ok(record)
}

pub fn revoke_punishment(
    store: &mut GuardStore,
    config: &GuardConfig,
    dirty: &mut bool,
    staff_id: &str,
    staff_name: &str,
    staff_rank: &str,
    target_name: &str,
    filter_kind: Option<PunishmentKind>,
    now_ms: u64,
) -> Result<usize, String> {
    let perms = config.perms_for(staff_rank);
    if staff_id != "CONSOLE" && !perms.can_unpunish {
        return Err("§c✗ У вашего ранга нет прав на снятие наказаний.".into());
    }

    let key = normalize_key(target_name);
    let mut revoked_count = 0usize;

    for rec in store.punishments.iter_mut() {
        if rec.target_key != key || !rec.is_active(now_ms) {
            continue;
        }
        let matches_kind = match filter_kind {
            None => true,
            Some(PunishmentKind::Ban) => rec.kind.is_ban(),
            Some(k) => rec.kind == k,
        };
        if matches_kind {
            rec.revoked_by = Some(staff_name.to_string());
            rec.revoked_at_ms = Some(now_ms);
            revoked_count += 1;
        }
    }

    if revoked_count > 0 {
        *dirty = true;
    }
    Ok(revoked_count)
}
