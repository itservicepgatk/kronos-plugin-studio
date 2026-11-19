use crate::hierarchy;
use crate::host;
use crate::models::{format_duration_ru, RanksConfig, RanksStore, StaffDepartment};
use serde::Serialize;
use std::collections::HashMap;

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
                material: "minecraft:black_stained_glass_pane".into(),
                name: " ".into(),
                lore: vec![],
                glow: false,
            });
        }
    }
}

/// GUI 1: Donate Showcase displaying all 10 donate ranks from VIP to KRONOS.
pub fn open_donate_showcase(config: &RanksConfig, store: &RanksStore, player_id: &str) {
    let name = store
        .id_to_name
        .get(player_id)
        .cloned()
        .unwrap_or_else(|| player_id.to_string());
    let key = hierarchy::normalize_key(&name);
    let current_donate = store
        .profiles
        .get(&key)
        .map(|p| p.donate_rank.as_str())
        .unwrap_or("player");

    let mut items = Vec::new();
    fill_border(&mut items, 5);

    let slots: [u8; 10] = [10, 11, 12, 13, 14, 15, 16, 20, 22, 24];
    let icons: [&str; 10] = [
        "minecraft:emerald",
        "minecraft:diamond",
        "minecraft:prismarine_shard",
        "minecraft:lapis_lazuli",
        "minecraft:amethyst_shard",
        "minecraft:gold_block",
        "minecraft:ender_eye",
        "minecraft:blaze_powder",
        "minecraft:netherite_ingot",
        "minecraft:nether_star",
    ];

    for (idx, rank) in config.donate_ranks().into_iter().take(10).enumerate() {
        let is_current = rank.id == current_donate;
        let p = &rank.perks;
        items.push(MenuItem {
            slot: slots[idx],
            material: icons[idx].into(),
            name: format!(
                "{} §8— §e{}₽{}",
                rank.prefix.trim(),
                rank.price_rub,
                if is_current { " §a✔ ВАШ" } else { "" }
            ),
            lore: vec![
                format!("§7Вес в табе: §f{}", rank.weight),
                format!("§7Дома (/sethome): §a{}", p.max_homes),
                format!(
                    "§7Приваты: §b{} §7(до §b{} §7бл.)",
                    p.max_regions, p.max_region_volume
                ),
                format!(
                    "§7Телепорт: §e{}с §8| §7RTP КД: §e{}с",
                    p.tp_warmup_seconds, p.rtp_cooldown_seconds
                ),
                format!("§7Множитель опыта: §dx{:.2}", p.xp_multiplier),
                "".into(),
                format!("§f{}", p.summary),
                "".into(),
                format!("§6Купить: §e{}", config.store_url),
            ],
            glow: is_current || rank.id == "kronos",
        });
    }

    items.push(MenuItem {
        slot: 38,
        material: "minecraft:player_head".into(),
        name: "§eМой профиль рангов".into(),
        lore: vec!["§7Посмотреть текущие ранги, срок и лимиты.".into()],
        glow: false,
    });
    items.push(MenuItem {
        slot: 40,
        material: "minecraft:shield".into(),
        name: "§bСтруктура Стаффа (9 должностей)".into(),
        lore: vec!["§7Посмотреть отделы администрации и онлайн.".into()],
        glow: false,
    });
    items.push(MenuItem {
        slot: 42,
        material: "minecraft:writable_book".into(),
        name: "§dХочешь в Стафф?".into(),
        lore: vec![
            "§7Подай заявку в Отдел Набора:".into(),
            "§e/staff apply <о себе и опыт>".into(),
        ],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        player_id,
        "kronos_ranks_donate",
        "§6§lKRONOS §8| §f10 Донат-привилегий",
        5,
        &json,
    );
}

/// GUI 2: Staff Structure grouped across all 9 department-based staff ranks.
pub fn open_staff_departments(
    config: &RanksConfig,
    store: &RanksStore,
    online_keys: &HashMap<String, String>,
    player_id: &str,
) {
    let mut items = Vec::new();
    fill_border(&mut items, 5);

    let slots: [u8; 9] = [10, 11, 12, 13, 14, 15, 16, 21, 23];
    for (idx, rank) in config.staff_ranks().into_iter().take(9).enumerate() {
        let online_members: Vec<String> = store
            .profiles
            .values()
            .filter(|p| p.staff_rank.as_deref() == Some(rank.id.as_str()) && online_keys.contains_key(&p.player_key))
            .map(|p| p.player_name.clone())
            .collect();

        let mat = match rank.department {
            StaffDepartment::Moderation => "minecraft:iron_sword",
            StaffDepartment::Recruitment => "minecraft:writable_book",
            StaffDepartment::Survival => "minecraft:grass_block",
            StaffDepartment::Technical => "minecraft:command_block",
            StaffDepartment::Management => "minecraft:beacon",
            StaffDepartment::None => "minecraft:paper",
        };

        let mut lore = vec![
            format!("§7Отдел: {}", rank.department.label_ru()),
            format!("§7Вес иерархии: §f{}", rank.weight),
            "".into(),
            format!("§f{}", rank.perks.summary),
            "".into(),
            format!("§7Сейчас онлайн: §a{}", online_members.len()),
        ];
        if !online_members.is_empty() {
            lore.push(format!("§e{}", online_members.join(", ")));
        }

        items.push(MenuItem {
            slot: slots[idx],
            material: mat.into(),
            name: format!("{} §8({})", rank.prefix.trim(), rank.display_name),
            lore,
            glow: !online_members.is_empty() || rank.id == "owner",
        });
    }

    items.push(MenuItem {
        slot: 38,
        material: "minecraft:nether_star".into(),
        name: "§6← К витрине 10 донат-рангов".into(),
        lore: vec![],
        glow: false,
    });
    items.push(MenuItem {
        slot: 40,
        material: "minecraft:bell".into(),
        name: "§dHR-Панель Отдела Набора".into(),
        lore: vec!["§7Доступно Куратору Набора, Гл.Куратору и Владельцу.".into()],
        glow: false,
    });
    items.push(MenuItem {
        slot: 42,
        material: "minecraft:book".into(),
        name: "§eЖурнал назначений (Аудит)".into(),
        lore: vec!["§7История выдачи и снятия рангов.".into()],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        player_id,
        "kronos_ranks_staff",
        "§b§lKRONOS Стафф §8| §f5 Отделов (9 рангов)",
        5,
        &json,
    );
}

/// GUI 3: Player Dual-Track Rank Profile.
pub fn open_player_profile(config: &RanksConfig, store: &RanksStore, viewer_id: &str, target_name: &str) {
    let now = host::now_ms();
    let key = hierarchy::normalize_key(target_name);
    let prof = store
        .profiles
        .get(&key)
        .cloned()
        .unwrap_or_else(|| crate::models::PlayerRankProfile::new(target_name, now));

    let primary = hierarchy::primary_rank(config, &prof);
    let perks = hierarchy::effective_perks(config, &prof);
    let donate_def = config.find(&prof.donate_rank).unwrap_or_else(|| config.default_rank());

    let donate_exp = match prof.donate_expires_at_ms {
        None => "§aНавсегда".into(),
        Some(ms) => format!("§e{}", format_duration_ru(ms.saturating_sub(now) / 1000)),
    };
    let staff_line = match &prof.staff_rank {
        None => "§7Нет должности".into(),
        Some(sid) => config
            .find(sid)
            .map(|r| format!("{} §8({})", r.prefix.trim(), r.department.label_ru()))
            .unwrap_or_else(|| sid.clone()),
    };

    let staff_exp = match (&prof.staff_rank, prof.staff_expires_at_ms) {
        (None, _) => "§8—".into(),
        (Some(_), None) => "§aПостоянная".into(),
        (Some(_), Some(ms)) => format!("§e{}", format_duration_ru(ms.saturating_sub(now) / 1000)),
    };

    let mut items = Vec::new();
    fill_border(&mut items, 4);

    items.push(MenuItem {
        slot: 11,
        material: "minecraft:nether_star".into(),
        name: format!("§6Донат-ветка: {}", donate_def.prefix.trim()),
        lore: vec![
            format!("§7Срок: {}", donate_exp),
            format!("§7Вес: §f{}", donate_def.weight),
        ],
        glow: prof.donate_rank != "player",
    });
    items.push(MenuItem {
        slot: 13,
        material: "minecraft:beacon".into(),
        name: format!("§bСтафф-ветка: {}", staff_line),
        lore: vec![
            format!("§7Срок должности: {}", staff_exp),
            format!("§7Активный префикс: {}", primary.prefix.trim()),
            format!("§7Итоговый вес: §e{}", primary.weight),
            "§7Обе ветки суммируют лучшие бонусы!".into(),
        ],
        glow: prof.staff_rank.is_some(),
    });
    items.push(MenuItem {
        slot: 15,
        material: "minecraft:chest".into(),
        name: "§aИтоговые лимиты игрока".into(),
        lore: vec![
            format!("§7Дома (/sethome): §a{}", perks.max_homes),
            format!("§7Приваты: §b{} §7(до §b{} §7бл.)", perks.max_regions, perks.max_region_volume),
            format!("§7TP задержка: §e{}с §8| §7RTP КД: §e{}с", perks.tp_warmup_seconds, perks.rtp_cooldown_seconds),
            format!("§7Опыт: §dx{:.2} §8| §7Сохранение XP: {}", perks.xp_multiplier, if perks.keep_xp_on_death { "§aДА" } else { "§cНЕТ" }),
        ],
        glow: false,
    });
    items.push(MenuItem {
        slot: 22,
        material: "minecraft:arrow".into(),
        name: "§7← Назад к донат-витрине".into(),
        lore: vec![],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        viewer_id,
        &format!("kronos_ranks_profile:{}", prof.player_name),
        &format!("§eПрофиль рангов: §f{}", prof.player_name),
        4,
        &json,
    );
}

/// GUI 4: HR Recruitment Panel for `hr_curator`, `curator`, and `owner`.
pub fn open_hr_panel(store: &RanksStore, staff_id: &str) {
    let mut items = Vec::new();
    fill_border(&mut items, 5);

    let pending: Vec<_> = store
        .applications
        .iter()
        .rev()
        .filter(|a| a.status == "pending")
        .take(21)
        .collect();
    let inner_slots: [u8; 21] = [
        10, 11, 12, 13, 14, 15, 16, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 31, 32, 33, 34,
    ];

    for (idx, app) in pending.into_iter().enumerate() {
        items.push(MenuItem {
            slot: inner_slots[idx],
            material: "minecraft:writable_book".into(),
            name: format!("§dЗаявка #{} §7— §f{}", app.id, app.applicant_name),
            lore: vec![
                format!("§7Анкета: §f{}", app.note),
                "".into(),
                "§aЛКМ: Принять на §e[Стажёр]".into(),
                "§cПКМ: Отклонить заявку".into(),
            ],
            glow: true,
        });
    }

    items.push(MenuItem {
        slot: 40,
        material: "minecraft:arrow".into(),
        name: "§7← Назад к отделам Стаффа".into(),
        lore: vec![],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        staff_id,
        "kronos_ranks_hr",
        "§d§lHR-Панель §8| §fЗаявки в Стафф",
        5,
        &json,
    );
}

/// GUI 5: Rank Assignment Audit Log.
pub fn open_audit_menu(store: &RanksStore, staff_id: &str) {
    let mut items = Vec::new();
    fill_border(&mut items, 5);

    let inner_slots: [u8; 21] = [
        10, 11, 12, 13, 14, 15, 16, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 31, 32, 33, 34,
    ];
    for (idx, entry) in store.audit_log.iter().rev().take(21).enumerate() {
        items.push(MenuItem {
            slot: inner_slots[idx],
            material: if entry.action == "SET" {
                "minecraft:emerald_block".into()
            } else {
                "minecraft:redstone_block".into()
            },
            name: format!("§e#{} §f{} §7→ §b{}", entry.id, entry.target_name, entry.rank_id),
            lore: vec![
                format!("§7Действие: §f{}", entry.action),
                format!("§7Выдал: §e{}", entry.actor_name),
                format!("§7Срок: §a{}", entry.duration_label),
            ],
            glow: false,
        });
    }

    items.push(MenuItem {
        slot: 40,
        material: "minecraft:arrow".into(),
        name: "§7← Назад к отделам Стаффа".into(),
        lore: vec![],
        glow: false,
    });

    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
    host::open_chest_menu(
        staff_id,
        "kronos_ranks_audit",
        "§e§lАудит иерархии рангов",
        5,
        &json,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn handle_menu_click(
    config: &RanksConfig,
    store: &mut RanksStore,
    online_keys: &HashMap<String, String>,
    dirty: &mut bool,
    player_id: &str,
    menu_id: &str,
    slot: u8,
    click_type: &str,
    refresh_online: fn(&RanksConfig, &RanksStore, &HashMap<String, String>, &str),
) -> bool {
    let viewer_name = store
        .id_to_name
        .get(player_id)
        .cloned()
        .unwrap_or_else(|| player_id.to_string());
    let viewer_key = hierarchy::normalize_key(&viewer_name);
    let now = host::now_ms();

    if menu_id == "kronos_ranks_donate" {
        match slot {
            38 => open_player_profile(config, store, player_id, &viewer_name),
            40 => open_staff_departments(config, store, online_keys, player_id),
            42 => host::send_chat(player_id, "§dИспользуйте: §f/staff apply <ваш возраст, опыт и отдел>"),
            _ => {}
        }
        return true;
    }

    if menu_id == "kronos_ranks_staff" {
        match slot {
            38 => open_donate_showcase(config, store, player_id),
            40 => {
                let allowed = store
                    .profiles
                    .get(&viewer_key)
                    .map(|p| hierarchy::has_permission(config, p, "kronos.staff.hr.panel"))
                    .unwrap_or(false);
                if allowed {
                    open_hr_panel(store, player_id);
                } else {
                    host::send_chat(player_id, "§c✗ HR-панель доступна Куратору Набора, Гл.Куратору и Владельцу.");
                }
            }
            42 => {
                let allowed = store
                    .profiles
                    .get(&viewer_key)
                    .map(|p| hierarchy::has_permission(config, p, "kronos.staff.hr.audit"))
                    .unwrap_or(false);
                if allowed {
                    open_audit_menu(store, player_id);
                } else {
                    host::send_chat(player_id, "§c✗ Аудит рангов доступен с должности §dКуратор Набора§c.");
                }
            }
            _ => {}
        }
        return true;
    }

    if menu_id.starts_with("kronos_ranks_profile:") {
        if slot == 22 {
            open_donate_showcase(config, store, player_id);
        }
        return true;
    }

    if menu_id == "kronos_ranks_hr" {
        if slot == 40 {
            open_staff_departments(config, store, online_keys, player_id);
            return true;
        }
        let inner_slots: [u8; 21] = [
            10, 11, 12, 13, 14, 15, 16, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 31, 32, 33, 34,
        ];
        if let Some(idx) = inner_slots.iter().position(|&s| s == slot) {
            let pending: Vec<(u64, String)> = store
                .applications
                .iter()
                .rev()
                .filter(|a| a.status == "pending")
                .map(|a| (a.id, a.applicant_name.clone()))
                .collect();
            if let Some((app_id, applicant)) = pending.get(idx) {
                if click_type.eq_ignore_ascii_case("right") {
                    if let Some(app) = store.applications.iter_mut().find(|a| a.id == *app_id) {
                        app.status = "rejected".into();
                        app.reviewed_by = Some(viewer_name);
                        *dirty = true;
                    }
                    host::send_chat(player_id, &format!("§7Заявка §f{} §cотклонена.", applicant));
                } else {
                    match hierarchy::assign_rank(
                        config,
                        store,
                        dirty,
                        Some(&viewer_key),
                        &viewer_name,
                        false,
                        applicant,
                        "trainee",
                        None,
                        now,
                    ) {
                        Ok(_) => {
                            if let Some(app) = store.applications.iter_mut().find(|a| a.id == *app_id) {
                                app.status = "accepted".into();
                                app.reviewed_by = Some(viewer_name.clone());
                            }
                            refresh_online(config, store, online_keys, applicant);
                            host::send_chat(
                                player_id,
                                &format!("§a✓ {} принят в Отдел Модерации на должность §e[Стажёр]§a!", applicant),
                            );
                        }
                        Err(e) => host::send_chat(player_id, &e),
                    }
                }
                open_hr_panel(store, player_id);
            }
        }
        return true;
    }

    if menu_id == "kronos_ranks_audit" {
        if slot == 40 {
            open_staff_departments(config, store, online_keys, player_id);
        }
        return true;
    }

    false
}
