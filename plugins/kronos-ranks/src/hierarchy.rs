use crate::models::{
    format_duration_ru, PlayerRankProfile, RankAuditEntry, RankCategory, RankDefinition, RankPerks,
    RanksConfig, RanksStore,
};
use std::collections::HashSet;

pub fn normalize_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Returns the highest-weight rank between the player's `staff_rank` and `donate_rank`.
pub fn primary_rank<'a>(config: &'a RanksConfig, profile: &PlayerRankProfile) -> &'a RankDefinition {
    let donate = config
        .find(&profile.donate_rank)
        .unwrap_or_else(|| config.default_rank());
    if let Some(ref staff_id) = profile.staff_rank {
        if let Some(staff) = config.find(staff_id) {
            if staff.weight >= donate.weight {
                return staff;
            }
        }
    }
    donate
}

/// Merges perks from BOTH `donate_rank` and `staff_rank` so staff members never lose their bought donate benefits!
pub fn effective_perks(config: &RanksConfig, profile: &PlayerRankProfile) -> RankPerks {
    let d = &config
        .find(&profile.donate_rank)
        .unwrap_or_else(|| config.default_rank())
        .perks;
    let Some(ref staff_id) = profile.staff_rank else {
        return d.clone();
    };
    let Some(s_def) = config.find(staff_id) else {
        return d.clone();
    };
    let s = &s_def.perks;

    RankPerks {
        max_homes: d.max_homes.max(s.max_homes),
        max_regions: d.max_regions.max(s.max_regions),
        max_region_volume: d.max_region_volume.max(s.max_region_volume),
        tp_warmup_seconds: d.tp_warmup_seconds.min(s.tp_warmup_seconds),
        rtp_cooldown_seconds: d.rtp_cooldown_seconds.min(s.rtp_cooldown_seconds),
        xp_multiplier: d.xp_multiplier.max(s.xp_multiplier),
        keep_xp_on_death: d.keep_xp_on_death || s.keep_xp_on_death,
        colored_chat: d.colored_chat || s.colored_chat,
        rgb_chat: d.rgb_chat || s.rgb_chat,
        custom_suffix: d.custom_suffix || s.custom_suffix,
        fly_in_own_region: d.fly_in_own_region || s.fly_in_own_region,
        border_tier: d.border_tier.max(s.border_tier),
        summary: s.summary.clone(),
    }
}

/// Checks if a permission pattern (like `kronos.guard.*` or `*`) matches a requested node.
pub fn permission_matches(pattern: &str, node: &str) -> bool {
    if pattern == "*" || pattern.eq_ignore_ascii_case(node) {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        let node_lower = node.to_lowercase();
        let pref_lower = prefix.to_lowercase();
        return node_lower == pref_lower || node_lower.starts_with(&format!("{}.", pref_lower));
    }
    false
}

/// Resolves all inherited permissions for a single rank ID (with cycle guard).
pub fn collect_rank_permissions(config: &RanksConfig, rank_id: &str) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut perms = Vec::new();
    let mut cursor = Some(rank_id.to_lowercase());

    while let Some(id) = cursor {
        if !visited.insert(id.clone()) {
            break;
        }
        if let Some(def) = config.find(&id) {
            perms.extend(def.permissions.iter().cloned());
            cursor = def.inherit_from.clone().map(|s| s.to_lowercase());
        } else {
            break;
        }
    }
    perms
}

/// Checks whether a player holds a permission across donate track, staff track, inheritance, and personal nodes.
pub fn has_permission(config: &RanksConfig, profile: &PlayerRankProfile, node: &str) -> bool {
    for p in &profile.extra_permissions {
        if permission_matches(p, node) {
            return true;
        }
    }
    for p in collect_rank_permissions(config, &profile.donate_rank) {
        if permission_matches(&p, node) {
            return true;
        }
    }
    if let Some(ref staff_id) = profile.staff_rank {
        for p in collect_rank_permissions(config, staff_id) {
            if permission_matches(&p, node) {
                return true;
            }
        }
    }
    false
}

/// Enforces department-based staff governance:
/// - Owner / CONSOLE can assign any rank.
/// - HR Curator can recruit/promote inside MODERATION up to `moder` (weight 220).
/// - Survival Admin can manage MODERATION/SURVIVAL up to `stmoder` (weight 300).
/// - Tech Admin can manage up to `survival_admin` (weight 550).
/// - Chief Curator can assign donate ranks and all staff ranks below Owner (weight 700).
pub fn can_assign_rank(
    config: &RanksConfig,
    actor_profile: Option<&PlayerRankProfile>,
    actor_is_console: bool,
    target_current: Option<&PlayerRankProfile>,
    new_rank: &RankDefinition,
) -> Result<(), String> {
    if actor_is_console {
        return Ok(());
    }
    let Some(actor) = actor_profile else {
        return Err("§c✗ Недостаточно прав для выдачи рангов.".into());
    };
    let actor_rank = primary_rank(config, actor);
    if actor_rank.id == "owner" || has_permission(config, actor, "*") {
        return Ok(());
    }

    // Cannot touch someone whose weight >= actor's weight
    if let Some(t_prof) = target_current {
        let t_rank = primary_rank(config, t_prof);
        if t_rank.weight >= actor_rank.weight {
            return Err(format!(
                "§c✗ Нельзя изменять ранг §f{} §c— его статус ({}) равен или выше вашего.",
                t_prof.player_name, t_rank.display_name
            ));
        }
    }

    if new_rank.weight > actor_rank.max_assignable_weight {
        return Err(format!(
            "§c✗ Ваш отдел (§e{}§c) может выдавать ранги до веса §e{}§c (у §f{}§c вес §e{}§c).",
            actor_rank.department.label_ru(),
            actor_rank.max_assignable_weight,
            new_rank.display_name,
            new_rank.weight
        ));
    }

    let dept_code = new_rank.department.code();
    let dept_allowed = actor_rank
        .assignable_departments
        .iter()
        .any(|d| d.eq_ignore_ascii_case(dept_code));

    // Donate ranks require explicit donate-grant permission or NONE department allowance
    if new_rank.category == RankCategory::Donate {
        if !has_permission(config, actor, "kronos.ranks.grant.donate") && !dept_allowed {
            return Err("§c✗ Выдача донат-привилегий доступна Гл.Куратору и Владельцу.".into());
        }
        return Ok(());
    }

    if !dept_allowed {
        return Err(format!(
            "§c✗ Вы не можете назначать сотрудников в отдел {}.",
            new_rank.department.label_ru()
        ));
    }

    Ok(())
}

/// Assigns a rank on the appropriate track (`donate` or `staff`) and writes an audit record.
#[allow(clippy::too_many_arguments)]
pub fn assign_rank(
    config: &RanksConfig,
    store: &mut RanksStore,
    dirty: &mut bool,
    actor_key: Option<&str>,
    actor_name: &str,
    actor_is_console: bool,
    target_name: &str,
    rank_id: &str,
    duration_secs: Option<u64>,
    now_ms: u64,
) -> Result<(PlayerRankProfile, RankDefinition), String> {
    let new_rank = config
        .find(rank_id)
        .cloned()
        .ok_or_else(|| format!("§c✗ Ранг §e{} §cне найден. Используйте §f/rank list§c.", rank_id))?;

    let target_key = normalize_key(target_name);
    let actor_prof = actor_key.and_then(|k| store.profiles.get(k)).cloned();
    let target_existing = store.profiles.get(&target_key).cloned();

    can_assign_rank(
        config,
        actor_prof.as_ref(),
        actor_is_console,
        target_existing.as_ref(),
        &new_rank,
    )?;

    let expires = duration_secs.map(|s| now_ms.saturating_add(s.saturating_mul(1000)));
    let profile = store
        .profiles
        .entry(target_key.clone())
        .or_insert_with(|| PlayerRankProfile::new(target_name, now_ms));
    profile.player_name = target_name.to_string();
    profile.updated_at_ms = now_ms;

    match new_rank.category {
        RankCategory::Staff => {
            profile.staff_rank = Some(new_rank.id.clone());
            profile.staff_expires_at_ms = expires;
        }
        RankCategory::Donate | RankCategory::Player => {
            profile.donate_rank = new_rank.id.clone();
            profile.donate_expires_at_ms = expires;
        }
    }

    let snapshot = profile.clone();
    let audit_id = store.next_audit_id.max(1);
    store.next_audit_id = audit_id + 1;
    let duration_label = match duration_secs {
        None => "навсегда".into(),
        Some(s) => format_duration_ru(s),
    };
    store.audit_log.push(RankAuditEntry {
        id: audit_id,
        actor_name: actor_name.to_string(),
        target_name: target_name.to_string(),
        action: "SET".into(),
        rank_id: new_rank.id.clone(),
        duration_label,
        ts_ms: now_ms,
    });
    if store.audit_log.len() > 500 {
        store.audit_log.remove(0);
    }

    *dirty = true;
    Ok((snapshot, new_rank))
}

/// Removes either the staff role or resets donate rank back to `player`.
pub fn remove_rank_track(
    config: &RanksConfig,
    store: &mut RanksStore,
    dirty: &mut bool,
    actor_key: Option<&str>,
    actor_name: &str,
    actor_is_console: bool,
    target_name: &str,
    track: &str,
    now_ms: u64,
) -> Result<PlayerRankProfile, String> {
    let target_key = normalize_key(target_name);
    let actor_prof = actor_key.and_then(|k| store.profiles.get(k)).cloned();
    let target_existing = store.profiles.get(&target_key).cloned();

    if !actor_is_console {
        let Some(ref actor) = actor_prof else {
            return Err("§c✗ Недостаточно прав.".into());
        };
        let actor_rank = primary_rank(config, actor);
        if actor_rank.id != "owner" {
            if let Some(ref t) = target_existing {
                if primary_rank(config, t).weight >= actor_rank.weight {
                    return Err("§c✗ Нельзя снять ранг у сотрудника выше или равного вам.".into());
                }
            }
            if track.eq_ignore_ascii_case("donate")
                && !has_permission(config, actor, "kronos.ranks.grant.donate")
            {
                return Err("§c✗ Снимать донат-ранги может только Гл.Куратор или Владелец.".into());
            }
            if track.eq_ignore_ascii_case("staff") && actor_rank.max_assignable_weight == 0 {
                return Err("§c✗ У вашего отдела нет прав на снятие должностей стаффа.".into());
            }
        }
    }

    let profile = store
        .profiles
        .entry(target_key)
        .or_insert_with(|| PlayerRankProfile::new(target_name, now_ms));

    let removed_id = if track.eq_ignore_ascii_case("donate") {
        let old = profile.donate_rank.clone();
        profile.donate_rank = "player".into();
        profile.donate_expires_at_ms = None;
        old
    } else {
        let old = profile.staff_rank.take().unwrap_or_else(|| "none".into());
        profile.staff_expires_at_ms = None;
        old
    };
    profile.updated_at_ms = now_ms;
    let snapshot = profile.clone();

    let audit_id = store.next_audit_id.max(1);
    store.next_audit_id = audit_id + 1;
    store.audit_log.push(RankAuditEntry {
        id: audit_id,
        actor_name: actor_name.to_string(),
        target_name: target_name.to_string(),
        action: format!("REMOVE_{}", track.to_uppercase()),
        rank_id: removed_id,
        duration_label: "-".into(),
        ts_ms: now_ms,
    });
    *dirty = true;
    Ok(snapshot)
}

/// Checks temporary donate/staff grants and reverts any expired ones.
/// Returns list of `(player_key, expired_rank_id)`.
pub fn expire_temp_ranks(store: &mut RanksStore, dirty: &mut bool, now_ms: u64) -> Vec<(String, String)> {
    let mut expired = Vec::new();
    for (key, prof) in store.profiles.iter_mut() {
        if let Some(exp) = prof.donate_expires_at_ms {
            if now_ms >= exp {
                let old = prof.donate_rank.clone();
                prof.donate_rank = "player".into();
                prof.donate_expires_at_ms = None;
                expired.push((key.clone(), old));
                *dirty = true;
            }
        }
        if let Some(exp) = prof.staff_expires_at_ms {
            if now_ms >= exp {
                if let Some(old) = prof.staff_rank.take() {
                    expired.push((key.clone(), old));
                }
                prof.staff_expires_at_ms = None;
                *dirty = true;
            }
        }
    }
    expired
}
