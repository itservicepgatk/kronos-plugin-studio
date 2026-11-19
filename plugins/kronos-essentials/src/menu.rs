use crate::effects::{can_use_theme, find_theme, THEME_SPECS};
use crate::host;
use crate::models::{
    ChestMenuDescriptor, CombatTag, EssentialsConfig, EssentialsStore, HomePoint, Location,
    MenuSlot, PendingTeleport, TpaRequest,
};
use std::collections::HashMap;

pub const HOME_GRID_SLOTS: [u8; 14] = [
    19, 20, 21, 22, 23, 24, 25,
    28, 29, 30, 31, 32, 33, 34,
];

const BORDER_SLOTS: [u8; 25] = [
    0, 1, 2, 3, 5, 6, 7, 8,
    9, 10, 11, 12, 13, 14, 15, 16, 17,
    18, 26, 27, 35, 36, 44,
    45, 53,
];

/// 1. Main 54-slot KRONOS Homes & Teleports Hub
pub fn open_homes_menu(
    store: &EssentialsStore,
    config: &EssentialsConfig,
    tpa_requests: &HashMap<String, TpaRequest>,
    combat_tags: &HashMap<String, CombatTag>,
    player_id: &str,
    page: usize,
) {
    let Some(player) = store.players.get(player_id) else {
        return;
    };
    let rank = config.effective_rank_for(player);
    let limit_text = if rank.unlimited_homes {
        "∞".to_string()
    } else {
        rank.max_homes.to_string()
    };
    let now = host::now_ms();
    let in_combat = combat_tags
        .get(player_id)
        .map(|c| c.expires_at_ms > now)
        .unwrap_or(false);
    let has_tpa = tpa_requests
        .get(player_id)
        .map(|r| r.expires_at_ms > now)
        .unwrap_or(false);

    let mut slots: Vec<MenuSlot> = Vec::with_capacity(54);

    for &slot in &BORDER_SLOTS {
        let is_corner = matches!(slot, 0 | 8 | 45 | 53);
        slots.push(MenuSlot {
            slot,
            item: if is_corner {
                "minecraft:orange_stained_glass_pane".to_string()
            } else {
                "minecraft:black_stained_glass_pane".to_string()
            },
            name: "§8".to_string(),
            lore: vec![],
            enchanted: false,
        });
    }

    // Top-center Player Status Card (slot 4)
    let active_theme = find_theme(player.active_theme(&rank));
    let fav_label = player
        .favorite_home()
        .map(|h| format!("§6★ {}", h.name))
        .unwrap_or_else(|| "§8не выбран".to_string());
    let next_hint = match config.next_donate_after(&rank) {
        Some(next) if !rank.unlimited_homes => format!(
            "§7Следующий донат {}: §e{} меток",
            next.display_prefix.trim(),
            next.max_homes
        ),
        _ => "§aМаксимальный лимит меток открыт!".to_string(),
    };

    slots.push(MenuSlot {
        slot: 4,
        item: "minecraft:player_head".to_string(),
        name: format!("{}§f{}", rank.display_prefix, player.username),
        lore: vec![
            format!("§7Точки дома: §e{} §8/ §a{}", player.homes.len(), limit_text),
            format!("§7Главный дом: {}", fav_label),
            format!("§7Задержка / Кулдаун: §f{}с §8/ §f{}с", rank.warmup_seconds, rank.cooldown_seconds),
            format!("§7Эффект: {}", active_theme.label),
            if in_combat {
                "§c⚔ Вы в бою! Телепорты временно заблокированы".to_string()
            } else {
                "§a✔ Статус: Безопасно".to_string()
            },
            "§8────────────────────".to_string(),
            next_hint,
        ],
        enchanted: true,
    });

    // 14 Home Slots
    let per_page = HOME_GRID_SLOTS.len();
    let offset = page * per_page;

    for (idx, &slot_num) in HOME_GRID_SLOTS.iter().enumerate() {
        let home_index = offset + idx;
        if let Some(home) = player.homes.get(home_index) {
            let star = if home.is_favorite { "§6★ " } else { "" };
            let shared_line = if home.shared_with.is_empty() {
                "§7Доступ: §8только вы".to_string()
            } else {
                format!("§7Друзья ({}): §b{}", home.shared_with.len(), home.shared_with.join(", "))
            };
            slots.push(MenuSlot {
                slot: slot_num,
                item: home.icon.clone(),
                name: format!("{}§6✦ §eДом: §f{}", star, home.name),
                lore: vec![
                    format!("§7Координаты: §f{}", home.location.coords_label()),
                    format!("§7Использований: §b{}", home.uses),
                    shared_line,
                    "§8────────────────────".to_string(),
                    "§a▸ ЛКМ §8— §fТелепортироваться".to_string(),
                    "§e▸ Shift+ЛКМ §8— §fНастройки дома (★, иконка, перенос)".to_string(),
                    "§c▸ ПКМ §8— §fУдалить (с подтверждением)".to_string(),
                ],
                enchanted: home.is_favorite,
            });
        } else if rank.unlimited_homes || home_index < rank.max_homes {
            slots.push(MenuSlot {
                slot: slot_num,
                item: "minecraft:lime_stained_glass_pane".to_string(),
                name: format!("§a+ Свободный слот #{}", home_index + 1),
                lore: vec![
                    "§7Нажмите, чтобы быстро создать".to_string(),
                    "§7точку дома на текущей позиции!".to_string(),
                    "§8Или введите: §f/sethome <название>".to_string(),
                ],
                enchanted: false,
            });
        } else {
            let unlock_rank = config.donate_unlock_for_slot(home_index);

            slots.push(MenuSlot {
                slot: slot_num,
                item: "minecraft:gray_stained_glass_pane".to_string(),
                name: "§8🔒 Заблокированный слот".to_string(),
                lore: vec![
                    format!("§7Доступно с ранга {}", unlock_rank),
                    "§7Донат-ветка (/donate): §e3 → 5 → 7 → ... → 50".to_string(),
                ],
                enchanted: false,
            });
        }
    }

    // Bottom Navigation Bar (Row 6)
    if page > 0 {
        slots.push(MenuSlot {
            slot: 45,
            item: "minecraft:arrow".to_string(),
            name: "§e◀ Предыдущая страница".to_string(),
            lore: vec![format!("§7Страница {}", page)],
            enchanted: false,
        });
    }

    slots.push(MenuSlot {
        slot: 46,
        item: "minecraft:firework_rocket".to_string(),
        name: "§6☄ Случайный телепорт (/rtp)".to_string(),
        lore: vec![
            "§7Безопасный поиск суши".to_string(),
            "§7+ метеоритный спуск с неба!".to_string(),
            "§a▸ Нажмите для запуска".to_string(),
        ],
        enchanted: true,
    });

    slots.push(MenuSlot {
        slot: 47,
        item: "minecraft:nether_star".to_string(),
        name: "§6✦ Телепорт на Спавн".to_string(),
        lore: vec![match &store.spawn {
            Some(loc) => format!("§7Точка: §f{}", loc.coords_label()),
            None => "§cСпавн не установлен (/setspawn)".to_string(),
        }],
        enchanted: store.spawn.is_some(),
    });

    slots.push(MenuSlot {
        slot: 48,
        item: "minecraft:compass".to_string(),
        name: "§b✦ Варпы Сервера".to_string(),
        lore: vec![format!("§7Доступно варпов: §e{}", store.warps.len())],
        enchanted: false,
    });

    slots.push(MenuSlot {
        slot: 49,
        item: "minecraft:ender_pearl".to_string(),
        name: "§d✦ Вернуться назад (/back)".to_string(),
        lore: vec![
            match &player.last_location {
                Some(loc) => format!("§7Прошлая точка: §f{}", loc.coords_label()),
                None => "§8Нет прошлой точки".to_string(),
            },
            if rank.can_back_on_death {
                match &player.last_death_location {
                    Some(d) => format!("§c☠ Точка смерти: §f{}", d.coords_label()),
                    None => "§7Возврат на точку смерти: §aдоступен".to_string(),
                }
            } else {
                "§8☠ Возврат на точку смерти: с VIP+".to_string()
            },
        ],
        enchanted: player.last_location.is_some() || (rank.can_back_on_death && player.last_death_location.is_some()),
    });

    slots.push(MenuSlot {
        slot: 50,
        item: "minecraft:amethyst_shard".to_string(),
        name: "§d✦ Меню эффектов телепорта".to_string(),
        lore: vec![
            format!("§7Активен: {}", active_theme.label),
            "§e▸ Открыть каталог анимаций".to_string(),
        ],
        enchanted: true,
    });

    slots.push(MenuSlot {
        slot: 51,
        item: "minecraft:bell".to_string(),
        name: if has_tpa {
            "§a✦ Входящий TPA-запрос! (1)".to_string()
        } else {
            "§7✦ TPA-запросы (0)".to_string()
        },
        lore: vec![if has_tpa {
            "§a▸ Нажмите, чтобы принять или отклонить".to_string()
        } else {
            "§8Нет активных входящих запросов".to_string()
        }],
        enchanted: has_tpa,
    });

    slots.push(MenuSlot {
        slot: 52,
        item: "minecraft:barrier".to_string(),
        name: "§cЗакрыть меню".to_string(),
        lore: vec![],
        enchanted: false,
    });

    if rank.unlimited_homes || (page + 1) * per_page < rank.max_homes {
        slots.push(MenuSlot {
            slot: 53,
            item: "minecraft:arrow".to_string(),
            name: "§eСледующая страница ▶".to_string(),
            lore: vec![format!("§7Страница {}", page + 2)],
            enchanted: false,
        });
    }

    let menu = ChestMenuDescriptor {
        menu_id: format!("kronos_homes:{}", page),
        title: format!("§8✦ §6KRONOS Телепорты §8[§e{}/{}§8]", player.homes.len(), limit_text),
        rows: 6,
        slots,
    };

    host::open_chest_menu(player_id, &menu);
    host::play_sound(player_id, "minecraft:ui.button.click", 0.6, 1.2);
}

/// 2. Confirm Delete Modal (27 slots) — prevents accidental right-click deletion
pub fn open_confirm_delete_menu(store: &EssentialsStore, player_id: &str, home_name: &str) {
    let Some(home) = store
        .players
        .get(player_id)
        .and_then(|p| p.homes.iter().find(|h| h.name == home_name))
    else {
        return;
    };

    let slots = vec![
        MenuSlot {
            slot: 11,
            item: "minecraft:lime_concrete".to_string(),
            name: "§a✔ Подтвердить удаление".to_string(),
            lore: vec![format!("§7Удалить точку дома §e{} §7навсегда", home.name)],
            enchanted: false,
        },
        MenuSlot {
            slot: 13,
            item: home.icon.clone(),
            name: format!("§6✦ Дом: §f{}", home.name),
            lore: vec![
                format!("§7Координаты: §f{}", home.location.coords_label()),
                format!("§7Использований: §b{}", home.uses),
            ],
            enchanted: true,
        },
        MenuSlot {
            slot: 15,
            item: "minecraft:red_concrete".to_string(),
            name: "§c✖ Отмена".to_string(),
            lore: vec!["§7Вернуться обратно к списку домов".to_string()],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("kronos_confirm_delete:{}", home.name),
            title: format!("§cУдалить дом {}?", home.name),
            rows: 3,
            slots,
        },
    );
}

/// 3. Home Settings Submenu (27 slots): Favorite ★, Move Here, Cycle Icon, Delete
pub fn open_home_edit_menu(store: &EssentialsStore, player_id: &str, home_name: &str) {
    let Some(home) = store
        .players
        .get(player_id)
        .and_then(|p| p.homes.iter().find(|h| h.name == home_name))
    else {
        return;
    };

    let slots = vec![
        MenuSlot {
            slot: 10,
            item: "minecraft:gold_ingot".to_string(),
            name: if home.is_favorite {
                "§6★ Главный дом (активен)".to_string()
            } else {
                "§e☆ Сделать главным домом".to_string()
            },
            lore: vec!["§7Отмечает дом звёздочкой и поднимает статус".to_string()],
            enchanted: home.is_favorite,
        },
        MenuSlot {
            slot: 12,
            item: "minecraft:ender_eye".to_string(),
            name: "§b✦ Переместить точку сюда".to_string(),
            lore: vec![
                format!("§7Сейчас: §f{}", home.location.coords_label()),
                "§a▸ Обновит координаты на вашу текущую позицию".to_string(),
            ],
            enchanted: false,
        },
        MenuSlot {
            slot: 14,
            item: home.icon.clone(),
            name: "§e✦ Сменить иконку".to_string(),
            lore: vec![
                format!("§7Текущая: §f{}", home.icon),
                "§a▸ Нажмите для переключения".to_string(),
            ],
            enchanted: false,
        },
        MenuSlot {
            slot: 16,
            item: "minecraft:tnt".to_string(),
            name: "§c✖ Удалить метку".to_string(),
            lore: vec!["§7Откроет окно подтверждения".to_string()],
            enchanted: false,
        },
        MenuSlot {
            slot: 22,
            item: "minecraft:arrow".to_string(),
            name: "§e◀ Назад к Домам".to_string(),
            lore: vec![],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("kronos_home_edit:{}", home.name),
            title: format!("§8Настройки дома: §6{}", home.name),
            rows: 3,
            slots,
        },
    );
}

/// 4. Rank-Locked Teleport Effects Catalog (27 slots)
pub fn open_effects_menu(store: &EssentialsStore, config: &EssentialsConfig, player_id: &str) {
    let Some(player) = store.players.get(player_id) else {
        return;
    };
    let rank = config.effective_rank_for(player);
    let active_id = player.active_theme(&rank);
    let effect_slots: [u8; 5] = [11, 12, 13, 14, 15];

    let mut slots: Vec<MenuSlot> = Vec::new();
    for (idx, theme) in THEME_SPECS.iter().enumerate() {
        let unlocked = rank.tier >= theme.min_tier;
        let is_selected = theme.id == active_id;
        slots.push(MenuSlot {
            slot: effect_slots[idx],
            item: if unlocked {
                theme.icon.to_string()
            } else {
                "minecraft:gray_dye".to_string()
            },
            name: if is_selected {
                format!("{} §a✔", theme.label)
            } else {
                theme.label.to_string()
            },
            lore: vec![
                format!("§7{}", theme.description),
                "§8────────────────────".to_string(),
                if is_selected {
                    "§aВыбрано сейчас".to_string()
                } else if unlocked {
                    "§e▸ Нажмите, чтобы активировать".to_string()
                } else {
                    format!(
                        "§c🔒 Требуется статус {}",
                        config.rank_name_for_tier(theme.min_tier)
                    )
                },
            ],
            enchanted: is_selected,
        });
    }

    slots.push(MenuSlot {
        slot: 22,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к Домам".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: "kronos_effects".to_string(),
            title: "§8✦ §dЭффекты телепортации".to_string(),
            rows: 3,
            slots,
        },
    );
}

/// 5. Incoming TPA Manager Menu (27 slots)
pub fn open_tpa_menu(tpa_requests: &HashMap<String, TpaRequest>, player_id: &str) {
    let now = host::now_ms();
    let mut slots: Vec<MenuSlot> = Vec::new();

    if let Some(req) = tpa_requests.get(player_id).filter(|r| r.expires_at_ms > now) {
        let rem = (req.expires_at_ms - now + 999) / 1000;
        let kind = if req.summon_here {
            "приглашает вас к себе"
        } else {
            "хочет телепортироваться к вам"
        };
        slots.push(MenuSlot {
            slot: 11,
            item: "minecraft:lime_concrete".to_string(),
            name: "§a✔ Принять запрос (/tpaccept)".to_string(),
            lore: vec![format!("§7Игрок §e{} §7{}", req.from_name, kind)],
            enchanted: true,
        });
        slots.push(MenuSlot {
            slot: 13,
            item: "minecraft:player_head".to_string(),
            name: format!("§6✦ Запрос от §e{}", req.from_name),
            lore: vec![
                format!("§7{}", kind),
                format!("§7Истекает через: §e{}с", rem),
            ],
            enchanted: false,
        });
        slots.push(MenuSlot {
            slot: 15,
            item: "minecraft:red_concrete".to_string(),
            name: "§c✖ Отклонить (/tpdeny)".to_string(),
            lore: vec![],
            enchanted: false,
        });
    } else {
        slots.push(MenuSlot {
            slot: 13,
            item: "minecraft:paper".to_string(),
            name: "§7Нет входящих TPA-запросов".to_string(),
            lore: vec!["§8Отправить запрос: §f/tpa <игрок>".to_string()],
            enchanted: false,
        });
    }

    slots.push(MenuSlot {
        slot: 22,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к Домам".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: "kronos_tpa".to_string(),
            title: "§8✦ §eВходящие TPA-запросы".to_string(),
            rows: 3,
            slots,
        },
    );
}

/// Server Warps Menu (54 slots)
pub fn open_warps_menu(store: &EssentialsStore, config: &EssentialsConfig, player_id: &str) {
    let Some(player) = store.players.get(player_id) else {
        return;
    };
    let rank = config.effective_rank_for(player);
    let mut slots: Vec<MenuSlot> = Vec::new();

    let mut sorted_warps: Vec<_> = store.warps.values().collect();
    sorted_warps.sort_by(|a, b| a.name.cmp(&b.name));

    for (idx, warp) in sorted_warps.iter().take(HOME_GRID_SLOTS.len()).enumerate() {
        let unlocked = rank.tier >= warp.min_tier;
        slots.push(MenuSlot {
            slot: HOME_GRID_SLOTS[idx],
            item: if unlocked {
                warp.icon.clone()
            } else {
                "minecraft:iron_bars".to_string()
            },
            name: format!("§b✦ Варп: §f{}", warp.name),
            lore: vec![
                format!("§7{}", warp.description),
                format!("§7Точка: §f{}", warp.location.coords_label()),
                if unlocked {
                    "§a▸ Нажмите для телепортации".to_string()
                } else {
                    format!("§c🔒 Требуется {}", config.rank_name_for_tier(warp.min_tier))
                },
            ],
            enchanted: unlocked,
        });
    }

    slots.push(MenuSlot {
        slot: 49,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к Домам".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: "kronos_warps".to_string(),
            title: "§8✦ §bKRONOS Варпы Сервера".to_string(),
            rows: 6,
            slots,
        },
    );
}

pub struct MenuDispatchContext<'a> {
    pub store: &'a mut EssentialsStore,
    pub config: &'a EssentialsConfig,
    pub pending: &'a mut HashMap<String, PendingTeleport>,
    pub tpa_requests: &'a mut HashMap<String, TpaRequest>,
    pub combat_tags: &'a HashMap<String, CombatTag>,
    pub dirty: &'a mut bool,
}

pub fn handle_menu_click(
    ctx: &mut MenuDispatchContext<'_>,
    player_id: &str,
    menu_id: &str,
    slot: u8,
    click_type: &str,
    schedule_tp: impl Fn(
        &mut EssentialsStore,
        &EssentialsConfig,
        &mut HashMap<String, PendingTeleport>,
        &HashMap<String, CombatTag>,
        &str,
        &str,
        Location,
        bool,
        &mut bool,
    ),
    trigger_rtp: impl Fn(
        &mut EssentialsStore,
        &EssentialsConfig,
        &mut HashMap<String, PendingTeleport>,
        &HashMap<String, CombatTag>,
        &str,
        &mut bool,
    ),
) -> bool {
    // Confirm Delete Modal
    if let Some(home_name) = menu_id.strip_prefix("kronos_confirm_delete:") {
        match slot {
            11 => {
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    let before = player.homes.len();
                    player.homes.retain(|h| h.name != home_name);
                    if player.homes.len() < before {
                        *ctx.dirty = true;
                        host::send_chat(player_id, &format!("§c✖ Дом §e{} §cудалён.", home_name));
                        host::play_sound(player_id, "minecraft:entity.item.break", 0.7, 1.0);
                    }
                }
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            15 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            _ => {}
        }
        return true;
    }

    // Home Settings Submenu
    if let Some(home_name) = menu_id.strip_prefix("kronos_home_edit:") {
        match slot {
            10 => {
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    for h in &mut player.homes {
                        h.is_favorite = h.name == home_name;
                    }
                    *ctx.dirty = true;
                    host::send_chat(player_id, &format!("§6★ Дом §e{} §6назначен главным!", home_name));
                    host::play_sound(player_id, "minecraft:entity.player.levelup", 0.65, 1.4);
                }
                open_home_edit_menu(ctx.store, player_id, home_name);
            }
            12 => {
                let new_loc = host::get_player_location(player_id);
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    if let Some(h) = player.homes.iter_mut().find(|h| h.name == home_name) {
                        h.location = new_loc.clone();
                        *ctx.dirty = true;
                        host::send_chat(
                            player_id,
                            &format!("§a✔ Координаты дома §e{} §aобновлены: §f{}", home_name, new_loc.coords_label()),
                        );
                        host::play_sound(player_id, "minecraft:entity.experience_orb.pickup", 0.7, 1.3);
                    }
                }
                open_home_edit_menu(ctx.store, player_id, home_name);
            }
            14 => {
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    if let Some(h) = player.homes.iter_mut().find(|h| h.name == home_name) {
                        if !ctx.config.home_icons.is_empty() {
                            let pos = ctx
                                .config
                                .home_icons
                                .iter()
                                .position(|i| i == &h.icon)
                                .unwrap_or(0);
                            h.icon = ctx.config.home_icons[(pos + 1) % ctx.config.home_icons.len()].clone();
                            *ctx.dirty = true;
                            host::play_sound(player_id, "minecraft:ui.button.click", 0.65, 1.3);
                        }
                    }
                }
                open_home_edit_menu(ctx.store, player_id, home_name);
            }
            16 => {
                open_confirm_delete_menu(ctx.store, player_id, home_name);
            }
            22 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            _ => {}
        }
        return true;
    }

    // Effects Catalog Menu
    if menu_id == "kronos_effects" {
        if slot == 22 {
            open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            return true;
        }
        let effect_slots: [u8; 5] = [11, 12, 13, 14, 15];
        if let Some(idx) = effect_slots.iter().position(|&s| s == slot) {
            let theme = &THEME_SPECS[idx];
            if let Some(player) = ctx.store.players.get_mut(player_id) {
                let rank = ctx.config.effective_rank_for(player);
                if !can_use_theme(rank.tier, theme.id) {
                    host::send_chat(
                        player_id,
                        &format!(
                            "§c🔒 Эффект {} §cдоступен с ранга {}",
                            theme.label,
                            ctx.config.rank_name_for_tier(theme.min_tier)
                        ),
                    );
                    host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
                } else {
                    player.selected_theme = Some(theme.id.to_string());
                    *ctx.dirty = true;
                    host::send_chat(player_id, &format!("§6✦ §fЭффект телепортации: {}", theme.label));
                    host::play_sound(player_id, theme.arrival_sound, 0.7, 1.2);
                }
            }
            open_effects_menu(ctx.store, ctx.config, player_id);
        }
        return true;
    }

    // Incoming TPA Menu
    if menu_id == "kronos_tpa" {
        match slot {
            11 => {
                let now = host::now_ms();
                if let Some(req) = ctx.tpa_requests.remove(player_id) {
                    if now <= req.expires_at_ms {
                        host::close_menu(player_id);
                        if req.summon_here {
                            let dest = host::get_player_location(&req.from_id);
                            schedule_tp(
                                ctx.store,
                                ctx.config,
                                ctx.pending,
                                ctx.combat_tags,
                                &req.to_id,
                                &format!("Игрок {}", req.from_name),
                                dest,
                                false,
                                ctx.dirty,
                            );
                        } else {
                            let dest = host::get_player_location(&req.to_id);
                            schedule_tp(
                                ctx.store,
                                ctx.config,
                                ctx.pending,
                                ctx.combat_tags,
                                &req.from_id,
                                &format!("Игрок {}", req.to_id),
                                dest,
                                false,
                                ctx.dirty,
                            );
                        }
                        host::send_chat(player_id, "§a✔ TPA-запрос принят!");
                    }
                }
            }
            15 => {
                if let Some(req) = ctx.tpa_requests.remove(player_id) {
                    host::send_chat(player_id, "§c✖ Запрос отклонён.");
                    host::send_chat(&req.from_id, &format!("§c✖ {} отклонил ваш TPA-запрос.", player_id));
                }
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            22 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            _ => {}
        }
        return true;
    }

    // Warps Menu
    if menu_id == "kronos_warps" {
        if slot == 49 {
            open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            return true;
        }
        if let Some(grid_idx) = HOME_GRID_SLOTS.iter().position(|&s| s == slot) {
            let mut sorted_warps: Vec<_> = ctx.store.warps.values().cloned().collect();
            sorted_warps.sort_by(|a, b| a.name.cmp(&b.name));
            if let Some(warp) = sorted_warps.get(grid_idx) {
                let rank = ctx
                    .store
                    .players
                    .get(player_id)
                    .map(|p| ctx.config.effective_rank_for(p))
                    .unwrap_or_else(|| ctx.config.ranks[0].clone());
                if rank.tier < warp.min_tier {
                    host::send_chat(player_id, "§c🔒 Недостаточный статус для этого варпа.");
                    host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 1.0);
                    return true;
                }
                host::close_menu(player_id);
                schedule_tp(
                    ctx.store,
                    ctx.config,
                    ctx.pending,
                    ctx.combat_tags,
                    player_id,
                    &format!("Варп {}", warp.name),
                    warp.location.clone(),
                    false,
                    ctx.dirty,
                );
            }
        }
        return true;
    }

    // Main Homes Hub
    if let Some(page_str) = menu_id.strip_prefix("kronos_homes:") {
        let page: usize = page_str.parse().unwrap_or(0);
        match slot {
            45 if page > 0 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, page - 1);
                return true;
            }
            53 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, page + 1);
                return true;
            }
            46 => {
                host::close_menu(player_id);
                trigger_rtp(ctx.store, ctx.config, ctx.pending, ctx.combat_tags, player_id, ctx.dirty);
                return true;
            }
            47 => {
                host::close_menu(player_id);
                if let Some(spawn) = ctx.store.spawn.clone() {
                    schedule_tp(
                        ctx.store,
                        ctx.config,
                        ctx.pending,
                        ctx.combat_tags,
                        player_id,
                        "Спавн",
                        spawn,
                        false,
                        ctx.dirty,
                    );
                } else {
                    host::send_chat(player_id, "§c✖ Спавн ещё не задан (/setspawn).");
                }
                return true;
            }
            48 => {
                open_warps_menu(ctx.store, ctx.config, player_id);
                return true;
            }
            49 => {
                host::close_menu(player_id);
                let target = ctx.store.players.get(player_id).and_then(|p| {
                    let rank = ctx.config.effective_rank_for(p);
                    if rank.can_back_on_death {
                        p.last_death_location.clone().or_else(|| p.last_location.clone())
                    } else {
                        p.last_location.clone()
                    }
                });
                if let Some(loc) = target {
                    schedule_tp(
                        ctx.store,
                        ctx.config,
                        ctx.pending,
                        ctx.combat_tags,
                        player_id,
                        "Прошлая точка",
                        loc,
                        false,
                        ctx.dirty,
                    );
                } else {
                    host::send_chat(player_id, "§c✖ Нет сохранённой точки для /back.");
                }
                return true;
            }
            50 => {
                open_effects_menu(ctx.store, ctx.config, player_id);
                return true;
            }
            51 => {
                open_tpa_menu(ctx.tpa_requests, player_id);
                return true;
            }
            52 => {
                host::close_menu(player_id);
                return true;
            }
            _ => {}
        }

        if let Some(grid_idx) = HOME_GRID_SLOTS.iter().position(|&s| s == slot) {
            let home_idx = page * HOME_GRID_SLOTS.len() + grid_idx;
            let Some(player) = ctx.store.players.get_mut(player_id) else {
                return true;
            };
            let rank = ctx.config.effective_rank_for(player);

            if home_idx < player.homes.len() {
                let home_name = player.homes[home_idx].name.clone();
                match click_type {
                    "right" => {
                        open_confirm_delete_menu(ctx.store, player_id, &home_name);
                    }
                    "shift_left" => {
                        open_home_edit_menu(ctx.store, player_id, &home_name);
                    }
                    _ => {
                        player.homes[home_idx].uses += 1;
                        let dest = player.homes[home_idx].location.clone();
                        *ctx.dirty = true;
                        host::close_menu(player_id);
                        schedule_tp(
                            ctx.store,
                            ctx.config,
                            ctx.pending,
                            ctx.combat_tags,
                            player_id,
                            &format!("Дом {}", home_name),
                            dest,
                            false,
                            ctx.dirty,
                        );
                    }
                }
            } else if rank.unlimited_homes || home_idx < rank.max_homes {
                let auto_name = format!("home{}", player.homes.len() + 1);
                let loc = host::get_player_location(player_id);
                let icon = ctx
                    .config
                    .home_icons
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "minecraft:red_bed".to_string());
                player.homes.push(HomePoint {
                    name: auto_name.clone(),
                    icon,
                    location: loc,
                    created_at_ms: host::now_ms(),
                    uses: 0,
                    is_favorite: player.homes.is_empty(),
                    shared_with: Vec::new(),
                });
                *ctx.dirty = true;
                host::send_chat(player_id, &format!("§a✔ Быстро создана метка §e{}§a!", auto_name));
                host::play_sound(player_id, "minecraft:entity.player.levelup", 0.65, 1.4);
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, page);
            } else {
                host::send_chat(
                    player_id,
                    "§c🔒 Слот заблокирован. Повысьте статус для расширения лимита домов!",
                );
                host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
            }
            return true;
        }
        return true;
    }

    false
}
