use crate::host;
use crate::models::{
    format_ago_ru, BlockAction, BlockLogEntry, GuardConfig, GuardStore, RollbackBatch, RollbackOp,
    RuntimeSession,
};
use std::collections::VecDeque;

/// Appends a compact block/container event to the bounded ring buffer.
#[allow(clippy::too_many_arguments)]
pub fn record_event(
    log: &mut VecDeque<BlockLogEntry>,
    config: &GuardConfig,
    dirty: &mut bool,
    world: &str,
    x: i32,
    y: i32,
    z: i32,
    actor: &str,
    action: BlockAction,
    block: &str,
    item: Option<String>,
    prev_block: Option<String>,
    now_ms: u64,
) {
    if !config.block_log.enabled {
        return;
    }

    let entry = BlockLogEntry {
        ts: now_ms / 1000,
        w: world.to_string(),
        x,
        y,
        z,
        actor: actor.to_string(),
        action,
        block: block.to_string(),
        item,
        prev_block,
        rolled_back: false,
    };

    log.push_back(entry);
    let max_entries = config.block_log.max_entries.max(100);
    while log.len() > max_entries {
        log.pop_front();
    }
    *dirty = true;
}

/// Removes entries older than `retention_days`.
pub fn prune_old_entries(
    log: &mut VecDeque<BlockLogEntry>,
    config: &GuardConfig,
    dirty: &mut bool,
    now_ms: u64,
) {
    let retention_secs = config.block_log.retention_days.max(1) * 86_400;
    let cutoff = (now_ms / 1000).saturating_sub(retention_secs);
    let mut removed = false;
    while let Some(front) = log.front() {
        if front.ts < cutoff {
            log.pop_front();
            removed = true;
        } else {
            break;
        }
    }
    if removed {
        *dirty = true;
    }
}

/// Looks up chronological history at an exact block/container coordinate (newest first).
pub fn lookup_at<'a>(
    log: &'a VecDeque<BlockLogEntry>,
    world: &str,
    x: i32,
    y: i32,
    z: i32,
    limit: usize,
) -> Vec<&'a BlockLogEntry> {
    log.iter()
        .rev()
        .filter(|e| e.w == world && e.x == x && e.y == y && e.z == z)
        .take(limit)
        .collect()
}

/// Looks up history within a cubic radius, optional time window (seconds), and optional actor filter.
#[allow(clippy::too_many_arguments)]
pub fn lookup_radius<'a>(
    log: &'a VecDeque<BlockLogEntry>,
    world: &str,
    cx: i32,
    cy: i32,
    cz: i32,
    radius: i32,
    since_sec: u64,
    actor_filter: Option<&str>,
    limit: usize,
) -> Vec<&'a BlockLogEntry> {
    let actor_lower = actor_filter.map(|s| s.to_lowercase());
    log.iter()
        .rev()
        .filter(|e| {
            e.w == world
                && e.ts >= since_sec
                && (e.x - cx).abs() <= radius
                && (e.y - cy).abs() <= radius
                && (e.z - cz).abs() <= radius
                && match &actor_lower {
                    None => true,
                    Some(a) => e.actor.to_lowercase() == *a,
                }
        })
        .take(limit)
        .collect()
}

pub fn format_entry_line(entry: &BlockLogEntry, now_sec: u64) -> String {
    let (sym, verb) = entry.action.symbol_ru();
    let ago = format_ago_ru(now_sec, entry.ts);
    let strike = if entry.rolled_back { "§m" } else { "" };
    let short_block = entry.block.strip_prefix("minecraft:").unwrap_or(&entry.block);

    if let Some(ref item_info) = entry.item {
        format!(
            "§8[§e{}§8] {} {}{} §7{} §e{} §8(§b{}§8) §8[{},{},{}]",
            ago, sym, strike, entry.actor, verb, short_block, item_info, entry.x, entry.y, entry.z
        )
    } else {
        format!(
            "§8[§e{}§8] {} {}{} §7{} §e{} §8[{},{},{}]",
            ago, sym, strike, entry.actor, verb, short_block, entry.x, entry.y, entry.z
        )
    }
}

/// Sends coordinate history to moderator when clicking a block/chest in Inspector mode (`/co i`).
pub fn inspect_block_for_staff(
    log: &VecDeque<BlockLogEntry>,
    config: &GuardConfig,
    session: &mut RuntimeSession,
    player_id: &str,
    world: &str,
    x: i32,
    y: i32,
    z: i32,
    now_ms: u64,
) {
    session
        .last_inspected_pos
        .insert(player_id.to_string(), (world.to_string(), x, y, z));

    let results = lookup_at(log, world, x, y, z, config.block_log.page_size.max(8));
    host::send_chat(
        player_id,
        &format!(
            "§8§m-----§r §6§lKRONOS Inspector §8({}: {}, {}, {}) §8§m-----",
            world, x, y, z
        ),
    );
    if results.is_empty() {
        host::send_chat(player_id, "§7Записей по этой координате не найдено.");
        host::play_sound(player_id, "minecraft:ui.button.click", 0.6, 0.8);
        return;
    }

    let now_sec = now_ms / 1000;
    for entry in results {
        host::send_chat(player_id, &format_entry_line(entry, now_sec));
    }
    host::play_sound(player_id, "minecraft:entity.experience_orb.pickup", 0.7, 1.4);
}

/// Rolls back block breaks, placements, explosions, and liquid griefing by a player in radius & time window.
#[allow(clippy::too_many_arguments)]
pub fn rollback_grief(
    log: &mut VecDeque<BlockLogEntry>,
    store: &mut GuardStore,
    config: &GuardConfig,
    session: &mut RuntimeSession,
    dirty_log: &mut bool,
    staff_id: &str,
    staff_name: &str,
    world: &str,
    cx: i32,
    cy: i32,
    cz: i32,
    radius: i32,
    since_sec: u64,
    actor_filter: Option<&str>,
    now_ms: u64,
) -> Result<usize, String> {
    let clamped_r = radius.clamp(1, config.block_log.max_rollback_radius);
    let actor_lower = actor_filter.map(|s| s.to_lowercase());
    let mut ops: Vec<RollbackOp> = Vec::new();

    for entry in log.iter_mut().rev() {
        if entry.rolled_back || entry.w != world || entry.ts < since_sec {
            continue;
        }
        if (entry.x - cx).abs() > clamped_r
            || (entry.y - cy).abs() > clamped_r
            || (entry.z - cz).abs() > clamped_r
        {
            continue;
        }
        if let Some(ref a) = actor_lower {
            if entry.actor.to_lowercase() != *a {
                continue;
            }
        }

        let target_restore = match entry.action {
            BlockAction::Break | BlockAction::Explosion => Some(entry.block.clone()),
            BlockAction::Place | BlockAction::LavaPlace | BlockAction::WaterPlace => {
                Some(entry.prev_block.clone().unwrap_or_else(|| "minecraft:air".into()))
            }
            BlockAction::ContainerOpen | BlockAction::ContainerTake | BlockAction::ContainerPut => {
                None
            }
        };

        if let Some(restore_block) = target_restore {
            if host::set_block(&entry.w, entry.x, entry.y, entry.z, &restore_block) {
                let after_block = match entry.action {
                    BlockAction::Break | BlockAction::Explosion => "minecraft:air".to_string(),
                    _ => entry.block.clone(),
                };
                ops.push(RollbackOp {
                    world: entry.w.clone(),
                    x: entry.x,
                    y: entry.y,
                    z: entry.z,
                    before_block: after_block,
                    after_block: restore_block,
                });
                entry.rolled_back = true;
            }
        }
    }

    let count = ops.len();
    if count > 0 {
        let batch_id = store.next_rollback_id.max(1);
        store.next_rollback_id = batch_id + 1;
        session
            .rollback_history
            .entry(staff_id.to_string())
            .or_default()
            .push(RollbackBatch {
                id: batch_id,
                staff_name: staff_name.to_string(),
                created_at_ms: now_ms,
                ops,
            });
        *dirty_log = true;
    }

    Ok(count)
}

/// Undoes the last rollback performed by this staff member.
pub fn undo_last_rollback(session: &mut RuntimeSession, staff_id: &str) -> Result<usize, String> {
    let stack = session
        .rollback_history
        .get_mut(staff_id)
        .ok_or_else(|| "§c✗ Нет откатов для отмены.".to_string())?;
    let batch = stack
        .pop()
        .ok_or_else(|| "§c✗ Нет откатов для отмены.".to_string())?;

    let mut restored = 0usize;
    for op in batch.ops.iter().rev() {
        if host::set_block(&op.world, op.x, op.y, op.z, &op.before_block) {
            restored += 1;
        }
    }
    Ok(restored)
}
