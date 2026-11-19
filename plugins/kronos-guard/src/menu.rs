use crate::host;
use crate::inspector;
use crate::models::{
    format_ago_ru, format_duration_ru, BlockAction, BlockLogEntry, GuardConfig, GuardStore,
    PunishmentKind, RuntimeSession,
};
use crate::punishments::{issue_punishment, normalize_key, revoke_punishment};
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Serialize)]
struct MenuItem {
    slot: u8,
    material: String,
    name: String,
    lore: Vec<String>,
    glow: bool,
}

fn fill_border(items: &mut Vec<MenuItem>, rows: u8) {
    let total = rows * 9;
    for slot in 0..total {
        let r = slot / 9;
        let c = slot % 9;
        if r == 0 || r == rows - 1 || c == 0 || c == 8 {
            items.push(MenuItem {
                slot,
                material: "minecraft:gray_stained_glass_pane".into(),
                name: " ".into(),
                lore: vec![],
                glow: false,
            });
        }
    }
}

pub fn open_guard_hub(
    store: &GuardStore,
    block_log: &VecDeque<BlockLogEntry>,
    config: &GuardConfig,
    session: &RuntimeSession,
    player_id: &str,
) {
    let now = host::now_ms();
    let is_vanished = session.vanished.contains(player_id);
    let is_inspecting = session.inspectors.contains(player_id);
    let open_reports = store.reports.iter().filter(|r| !r.resolved).count();
    let active_bans = store
        .punishments
        .iter()
        .filter(|p| p.kind.is_ban() && p.is_active(now))
        .count();
    let active_mutes = store
        .punishments
        .iter()
        .filter(|p| p.kind == PunishmentKind::Mute && p.is_active(now))
        .count();

    let mut items = Vec::new();
    fill_border(&mut items, 4);

    // Slot 11: Vanish toggle
    items.push(MenuItem {
        slot: 11,
        material: if is_vanished {
            "minecraft:ender_eye".into()
        } else {
            "minecraft:ender_pearl".into()
        },
        name: if is_vanished {
            "§b§lВаниш: ВКЛЮЧЁН".into()
        } else {
            "§7Ваниш: ВЫКЛЮЧЕН".into()
        },
        lore: vec![
            "§7Полная невидимость для игроков".into(),
            "§7ниже вашего ранга + тихие сундуки.".into(),
            "".into(),
            "§eЛКМ: Переключить /vanish".into(),
        ],
        glow: is_vanished,
    });

    // Slot 13: Block Inspector toggle
    items.push(MenuItem {
        slot: 13,
        material: config.inspector_item.clone(),
        name: if is_inspecting {
            "§6§lИнспектор блоков: АКТИВЕН".into()
        } else {
            "§eИнспектор блоков: ВЫКЛЮЧЕН".into()
        },
        lore: vec![
            format!("§7Записей в журнале: §f{}", block_log.len()),
            "§7Клик по блоку или сундуку покажет,".into(),
            "§7кто его сломал, поставил или открыл.".into(),
            "".into(),
            "§eЛКМ: Переключить /co i".into(),
            "§bПКМ: Последняя проверенная точка".into(),
        ],
        glow: is_inspecting,
    });

    // Slot 15: Reports Queue
    items.push(MenuItem {
        slot: 15,
        material: if open_reports > 0 {
            "minecraft:bell".into()
        } else {
            "minecraft:book".into()
        },
        name: format!("§c§lЖалобы игроков §8(§e{}§8)", open_reports),
        lore: vec![
            "§7Очередь репортов от /report.".into(),
            format!("§7Неразобранных: §e{}", open_reports),
            "".into(),
            "§eЛКМ: Открыть очередь жалоб".into(),
        ],
        glow: open_reports > 0,
    });

    // Slot 20: Active Stats
    items.push(MenuItem {
        slot: 20,
        material: "minecraft:iron_bars".into(),
        name: "§cСтатистика наказаний".into(),
        lore: vec![
            format!("§7Активных банов: §c{}", active_bans),
            format!("§7Активных мутов: §6{}", active_mutes),
            format!("§7Всего в базе: §f{}", store.punishments.len()),
            format!("§7Заморожено сейчас: §b{}", session.frozen.len()),
        ],
        glow: false,
    });

    // Slot 22: Chat Guard info
    items.push(MenuItem {
        slot: 22,
        material: "minecraft:shield".into(),
        name: "§a§lАнти-Спам & Анти-Реклама".into(),
        lore: vec![
            format!(
                "§7Статус: {}",
                if config.chat_filter.enabled {
                    "§aВКЛ"
                } else {
                    "§cВЫКЛ"
                }
            ),
            format!(
                "§7Кулдаун чата: §f{} мс",
                config.chat_filter.cooldown_ms
            ),
            format!(
                "§7Авто-мут после §e{} §7страйков (§e{}§7)",
                config.chat_filter.auto_mute_strikes, config.chat_filter.auto_mute_duration
            ),
            "§7Блокирует IP, ссылки, TG/DS, флуд и мат.".into(),
        ],
        glow: config.chat_filter.enabled,
    });

    // Slot 24: Online Players Quick Moderation
    items.push(MenuItem {
        slot: 24,
        material: "minecraft:player_head".into(),
        name: format!("§eИгроки онлайн §8(§a{}§8)", session.online_ids.len()),
        lore: vec![
            "§7Откройте панель быстрого наказания".into(),
            "§7командой §e/guard player <ник>".into(),
            "§7или §e/history <ник>§7.".into(),
        ],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        player_id,
        "kronos_guard_hub",
        "§4§lKRONOS Guard §8| §fМодерация",
        4,
        &json,
    );
}

pub fn open_player_panel(
    store: &GuardStore,
    session: &RuntimeSession,
    staff_id: &str,
    target_name: &str,
) {
    let now = host::now_ms();
    let key = normalize_key(target_name);
    let is_frozen = session.frozen.contains_key(&key);
    let warns = store
        .punishments
        .iter()
        .filter(|p| p.target_key == key && p.kind == PunishmentKind::Warn && p.is_active(now))
        .count();

    let mut items = Vec::new();
    fill_border(&mut items, 4);

    // Quick mute 15m
    items.push(MenuItem {
        slot: 10,
        material: "minecraft:paper".into(),
        name: "§6Мут 15 минут".into(),
        lore: vec!["§7Причина: Флуд / Спам в чате".into(), "§eЛКМ: Выдать".into()],
        glow: false,
    });
    // Quick mute 1h
    items.push(MenuItem {
        slot: 11,
        material: "minecraft:writable_book".into(),
        name: "§6Мут 1 час".into(),
        lore: vec!["§7Причина: Оскорбления / мат".into(), "§eЛКМ: Выдать".into()],
        glow: false,
    });
    // Quick warn
    items.push(MenuItem {
        slot: 12,
        material: "minecraft:gold_ingot".into(),
        name: format!("§eВыдать Варн §8(сейчас: {})", warns),
        lore: vec![
            "§72 варна → авто-мут 30м".into(),
            "§73 варна → авто-бан 24ч".into(),
            "§eЛКМ: Выдать предупреждение".into(),
        ],
        glow: false,
    });
    // Quick tempban 1d
    items.push(MenuItem {
        slot: 14,
        material: "minecraft:iron_axe".into(),
        name: "§cТемпбан 24 часа".into(),
        lore: vec!["§7Причина: Гриферство / Нарушение правил".into(), "§eЛКМ: Выдать".into()],
        glow: false,
    });
    // Quick tempban 7d
    items.push(MenuItem {
        slot: 15,
        material: "minecraft:diamond_axe".into(),
        name: "§c§lТемпбан 7 дней".into(),
        lore: vec!["§7Причина: Тяжёлый гриф / Подозрение на читы".into(), "§eЛКМ: Выдать".into()],
        glow: false,
    });
    // Permanent Ban
    items.push(MenuItem {
        slot: 16,
        material: "minecraft:netherite_axe".into(),
        name: "§4§lПерманентный БАН".into(),
        lore: vec!["§7Требуется ранг §3Ст.Модер+".into(), "§cЛКМ: Забанить навсегда".into()],
        glow: true,
    });

    // Row 2: Freeze, History, Unpunish, Back
    items.push(MenuItem {
        slot: 20,
        material: "minecraft:packed_ice".into(),
        name: if is_frozen {
            "§b§lРазморозить игрока".into()
        } else {
            "§bЗаморозить на проверку (/freeze)".into()
        },
        lore: vec![
            "§7Выход при заморозке → авто-бан 7д.".into(),
            "§eЛКМ: Переключить заморозку".into(),
        ],
        glow: is_frozen,
    });
    items.push(MenuItem {
        slot: 22,
        material: "minecraft:book".into(),
        name: "§eИстория наказаний".into(),
        lore: vec!["§7Открыть все баны, муты и варны игрока.".into()],
        glow: false,
    });
    items.push(MenuItem {
        slot: 24,
        material: "minecraft:milk_bucket".into(),
        name: "§aСнять активные наказания".into(),
        lore: vec!["§7Снимает активные баны/муты игрока.".into()],
        glow: false,
    });
    items.push(MenuItem {
        slot: 31,
        material: "minecraft:arrow".into(),
        name: "§7← Назад в панель Guard".into(),
        lore: vec![],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        staff_id,
        &format!("kronos_guard_player:{}", target_name),
        &format!("§cМодерация: §f{}", target_name),
        4,
        &json,
    );
}

pub fn open_history_menu(store: &GuardStore, staff_id: &str, target_name: &str) {
    let now = host::now_ms();
    let key = normalize_key(target_name);
    let mut items = Vec::new();
    fill_border(&mut items, 5);

    let history: Vec<_> = store
        .punishments
        .iter()
        .rev()
        .filter(|p| p.target_key == key)
        .take(21)
        .collect();

    let inner_slots: [u8; 21] = [
        10, 11, 12, 13, 14, 15, 16, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 31, 32, 33, 34,
    ];

    for (idx, rec) in history.into_iter().enumerate() {
        let active = rec.is_active(now);
        let status = if rec.revoked_at_ms.is_some() {
            format!("§7Снято ({})", rec.revoked_by.as_deref().unwrap_or("Staff"))
        } else if active {
            match rec.remaining_ms(now) {
                None => "§4АКТИВНО (Навсегда)".into(),
                Some(ms) => format!("§cАКТИВНО ({})", format_duration_ru(ms / 1000)),
            }
        } else {
            "§8Истекло".into()
        };

        items.push(MenuItem {
            slot: inner_slots[idx],
            material: match rec.kind {
                PunishmentKind::Ban | PunishmentKind::IpBan => "minecraft:redstone_block".into(),
                PunishmentKind::TempBan => "minecraft:red_concrete".into(),
                PunishmentKind::Mute => "minecraft:orange_concrete".into(),
                PunishmentKind::Warn => "minecraft:yellow_concrete".into(),
                PunishmentKind::Kick => "minecraft:light_gray_concrete".into(),
            },
            name: format!("{} §8#{}", rec.kind.label_ru(), rec.id),
            lore: vec![
                format!("§7Статус: {}", status),
                format!("§7Модератор: §e{}", rec.staff_name),
                format!("§7Причина: §f{}", rec.reason),
            ],
            glow: active,
        });
    }

    items.push(MenuItem {
        slot: 40,
        material: "minecraft:arrow".into(),
        name: "§7← Назад к карточке игрока".into(),
        lore: vec![],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        staff_id,
        &format!("kronos_guard_history:{}", target_name),
        &format!("§eИстория: §f{}", target_name),
        5,
        &json,
    );
}

pub fn open_reports_menu(store: &GuardStore, staff_id: &str) {
    let now_sec = host::now_ms() / 1000;
    let mut items = Vec::new();
    fill_border(&mut items, 5);

    let open: Vec<_> = store.reports.iter().rev().filter(|r| !r.resolved).take(21).collect();
    let inner_slots: [u8; 21] = [
        10, 11, 12, 13, 14, 15, 16, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 31, 32, 33, 34,
    ];

    for (idx, rep) in open.into_iter().enumerate() {
        items.push(MenuItem {
            slot: inner_slots[idx],
            material: "minecraft:paper".into(),
            name: format!("§cЖалоба #{} §7на §f{}", rep.id, rep.target_name),
            lore: vec![
                format!("§7От: §e{}", rep.reporter_name),
                format!("§7Причина: §f{}", rep.reason),
                format!("§7Координаты: §b{} [{},{},{}]", rep.world, rep.x, rep.y, rep.z),
                format!("§7Когда: §7{}", format_ago_ru(now_sec, rep.created_at_ms / 1000)),
                "".into(),
                "§eЛКМ: Открыть панель нарушителя".into(),
                "§aПКМ: Отметить жалобу закрытой".into(),
            ],
            glow: true,
        });
    }

    items.push(MenuItem {
        slot: 40,
        material: "minecraft:arrow".into(),
        name: "§7← Назад в панель Guard".into(),
        lore: vec![],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        staff_id,
        "kronos_guard_reports",
        "§c§lОчередь жалоб (/report)",
        5,
        &json,
    );
}

pub fn open_blocklog_menu(
    block_log: &VecDeque<BlockLogEntry>,
    staff_id: &str,
    world: &str,
    x: i32,
    y: i32,
    z: i32,
) {
    let now_sec = host::now_ms() / 1000;
    let mut items = Vec::new();
    fill_border(&mut items, 5);

    let entries = inspector::lookup_at(block_log, world, x, y, z, 21);
    let inner_slots: [u8; 21] = [
        10, 11, 12, 13, 14, 15, 16, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 31, 32, 33, 34,
    ];

    for (idx, entry) in entries.into_iter().enumerate() {
        let (sym, verb) = entry.action.symbol_ru();
        let mat = match entry.action {
            BlockAction::Break => "minecraft:iron_pickaxe",
            BlockAction::Place => "minecraft:grass_block",
            BlockAction::ContainerOpen | BlockAction::ContainerTake | BlockAction::ContainerPut => {
                "minecraft:chest"
            }
            BlockAction::Explosion => "minecraft:tnt",
            BlockAction::LavaPlace => "minecraft:lava_bucket",
            BlockAction::WaterPlace => "minecraft:water_bucket",
        };
        let mut lore = vec![
            format!("§7Игрок: §f{}", entry.actor),
            format!("§7Действие: {} §7{}", sym, verb),
            format!("§7Блок: §e{}", entry.block),
            format!("§7Время: §b{}", format_ago_ru(now_sec, entry.ts)),
        ];
        if let Some(ref item) = entry.item {
            lore.push(format!("§7Предмет: §a{}", item));
        }
        if entry.rolled_back {
            lore.push("§8[Уже откачено]".into());
        }

        items.push(MenuItem {
            slot: inner_slots[idx],
            material: mat.into(),
            name: format!("{} §f{}", sym, entry.actor),
            lore,
            glow: !entry.rolled_back && entry.action == BlockAction::Break,
        });
    }

    items.push(MenuItem {
        slot: 40,
        material: "minecraft:arrow".into(),
        name: "§7← Назад в панель Guard".into(),
        lore: vec![],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        staff_id,
        "kronos_guard_blocklog",
        &format!("§6Журнал блока [{},{},{}]", x, y, z),
        5,
        &json,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn handle_menu_click(
    store: &mut GuardStore,
    block_log: &VecDeque<BlockLogEntry>,
    config: &GuardConfig,
    session: &mut RuntimeSession,
    dirty_store: &mut bool,
    staff_id: &str,
    menu_id: &str,
    slot: u8,
    click_type: &str,
    toggle_vanish: fn(&GuardConfig, &GuardStore, &mut RuntimeSession, &str),
    toggle_inspector: fn(&GuardConfig, &mut RuntimeSession, &str),
    toggle_freeze: fn(&GuardConfig, &GuardStore, &mut RuntimeSession, &str, &str),
) -> bool {
    let staff_rank = host::get_player_rank(staff_id);
    let staff_name = store
        .known_names
        .get(staff_id)
        .cloned()
        .unwrap_or_else(|| staff_id.to_string());
    let now = host::now_ms();

    if menu_id == "kronos_guard_hub" {
        match slot {
            11 => {
                toggle_vanish(config, store, session, staff_id);
                open_guard_hub(store, block_log, config, session, staff_id);
            }
            13 => {
                if click_type.eq_ignore_ascii_case("right") {
                    if let Some((w, x, y, z)) = session.last_inspected_pos.get(staff_id).cloned() {
                        open_blocklog_menu(block_log, staff_id, &w, x, y, z);
                    } else {
                        let pos = host::get_player_pos(staff_id);
                        open_blocklog_menu(block_log, staff_id, &pos.world, pos.x, pos.y, pos.z);
                    }
                } else {
                    toggle_inspector(config, session, staff_id);
                    open_guard_hub(store, block_log, config, session, staff_id);
                }
            }
            15 => open_reports_menu(store, staff_id),
            _ => {}
        }
        return true;
    }

    if let Some(target_name) = menu_id.strip_prefix("kronos_guard_player:") {
        let target = target_name.to_string();
        match slot {
            10 => {
                let res = issue_punishment(
                    store,
                    config,
                    session,
                    dirty_store,
                    staff_id,
                    &staff_name,
                    &staff_rank,
                    PunishmentKind::Mute,
                    &target,
                    Some(900),
                    "Флуд / спам в чате",
                    now,
                );
                if let Err(e) = res {
                    host::send_chat(staff_id, &e);
                }
                open_player_panel(store, session, staff_id, &target);
            }
            11 => {
                let res = issue_punishment(
                    store,
                    config,
                    session,
                    dirty_store,
                    staff_id,
                    &staff_name,
                    &staff_rank,
                    PunishmentKind::Mute,
                    &target,
                    Some(3600),
                    "Оскорбления / мат",
                    now,
                );
                if let Err(e) = res {
                    host::send_chat(staff_id, &e);
                }
                open_player_panel(store, session, staff_id, &target);
            }
            12 => {
                let res = issue_punishment(
                    store,
                    config,
                    session,
                    dirty_store,
                    staff_id,
                    &staff_name,
                    &staff_rank,
                    PunishmentKind::Warn,
                    &target,
                    None,
                    "Предупреждение модератора",
                    now,
                );
                if let Err(e) = res {
                    host::send_chat(staff_id, &e);
                }
                open_player_panel(store, session, staff_id, &target);
            }
            14 => {
                let res = issue_punishment(
                    store,
                    config,
                    session,
                    dirty_store,
                    staff_id,
                    &staff_name,
                    &staff_rank,
                    PunishmentKind::TempBan,
                    &target,
                    Some(86_400),
                    "Гриферство / нарушение правил",
                    now,
                );
                if let Err(e) = res {
                    host::send_chat(staff_id, &e);
                }
            }
            15 => {
                let res = issue_punishment(
                    store,
                    config,
                    session,
                    dirty_store,
                    staff_id,
                    &staff_name,
                    &staff_rank,
                    PunishmentKind::TempBan,
                    &target,
                    Some(604_800),
                    "Тяжёлый гриф / читы",
                    now,
                );
                if let Err(e) = res {
                    host::send_chat(staff_id, &e);
                }
            }
            16 => {
                let res = issue_punishment(
                    store,
                    config,
                    session,
                    dirty_store,
                    staff_id,
                    &staff_name,
                    &staff_rank,
                    PunishmentKind::Ban,
                    &target,
                    None,
                    "Грубое нарушение правил сервера",
                    now,
                );
                if let Err(e) = res {
                    host::send_chat(staff_id, &e);
                }
            }
            20 => {
                toggle_freeze(config, store, session, staff_id, &target);
                open_player_panel(store, session, staff_id, &target);
            }
            22 => open_history_menu(store, staff_id, &target),
            24 => {
                match revoke_punishment(
                    store,
                    config,
                    dirty_store,
                    staff_id,
                    &staff_name,
                    &staff_rank,
                    &target,
                    None,
                    now,
                ) {
                    Ok(cnt) => host::send_chat(
                        staff_id,
                        &format!("§a✓ Снято активных наказаний с §f{}§a: §e{}", target, cnt),
                    ),
                    Err(e) => host::send_chat(staff_id, &e),
                }
                open_player_panel(store, session, staff_id, &target);
            }
            31 => open_guard_hub(store, block_log, config, session, staff_id),
            _ => {}
        }
        return true;
    }

    if let Some(target_name) = menu_id.strip_prefix("kronos_guard_history:") {
        if slot == 40 {
            open_player_panel(store, session, staff_id, target_name);
        }
        return true;
    }

    if menu_id == "kronos_guard_reports" {
        if slot == 40 {
            open_guard_hub(store, block_log, config, session, staff_id);
            return true;
        }
        let inner_slots: [u8; 21] = [
            10, 11, 12, 13, 14, 15, 16, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 31, 32, 33, 34,
        ];
        if let Some(pos) = inner_slots.iter().position(|&s| s == slot) {
            let open_ids: Vec<(u64, String)> = store
                .reports
                .iter()
                .rev()
                .filter(|r| !r.resolved)
                .map(|r| (r.id, r.target_name.clone()))
                .collect();
            if let Some((rep_id, target)) = open_ids.get(pos) {
                if click_type.eq_ignore_ascii_case("right") {
                    if let Some(rep) = store.reports.iter_mut().find(|r| r.id == *rep_id) {
                        rep.resolved = true;
                        rep.resolved_by = Some(staff_name);
                        *dirty_store = true;
                    }
                    open_reports_menu(store, staff_id);
                } else {
                    open_player_panel(store, session, staff_id, target);
                }
            }
        }
        return true;
    }

    if menu_id == "kronos_guard_blocklog" {
        if slot == 40 {
            open_guard_hub(store, block_log, config, session, staff_id);
        }
        return true;
    }

    false
}
