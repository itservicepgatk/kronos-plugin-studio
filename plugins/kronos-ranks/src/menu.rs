use std::collections::{HashMap, HashSet};

use pumpkin_plugin_api::{
    text::TextComponent,
    wit::pumpkin::plugin::forms::{Form, SimpleForm, SimpleFormButton},
    Player,
};

use crate::hierarchy::{effective_perks, normalize_key, primary_rank};
use crate::models::{
    format_duration_ru, BedrockRankAction, PendingBedrockRankForm, PlayerRankProfile,
    RankAuditEntry, RankDefinition, RanksConfig, StaffApplication,
};
use crate::storage::with_state;
use crate::tab_chat::{legacy, play_click_sound, resolved_suffix};

const LINE_SEP: &str = "§8§m────────────────────────────────────────────";

pub fn clickable_run(label: &str, command: &str, hover: &str) -> TextComponent {
    legacy(label)
        .click_run_command(command)
        .hover_show_text(legacy(hover))
}

pub fn clickable_suggest(label: &str, command: &str, hover: &str) -> TextComponent {
    legacy(label)
        .click_suggest_command(command)
        .hover_show_text(legacy(hover))
}

fn open_bedrock_form(
    player: &Player,
    player_name: &str,
    title: &str,
    content: &str,
    buttons: Vec<SimpleFormButton>,
    actions: Vec<BedrockRankAction>,
) {
    if let Some(bedrock) = player.as_bedrock() {
        let form = Form::Simple(SimpleForm {
            title: legacy(title),
            content: legacy(content),
            buttons,
        });
        let form_id = bedrock.open_form(form);
        let player_key = normalize_key(player_name);
        with_state(|st| {
            st.pending_forms.insert(
                form_id,
                PendingBedrockRankForm {
                    player_key,
                    actions,
                },
            );
        });
    }
}

/// Opens the interactive Donate Showcase (`/donate`, `/ranks`) for all 10 donate tiers.
/// - Java players receive clickable `TextComponent` rows with hover tooltips.
/// - Bedrock players also receive a native `SimpleForm`.
pub fn open_donate_showcase(
    player: &Player,
    config: &RanksConfig,
    viewer_profile: &PlayerRankProfile,
) {
    play_click_sound(player);
    let viewer_name = player.get_name();
    let donate_ranks = config.donate_ranks();

    // Bedrock native SimpleForm
    if player.as_bedrock().is_some() {
        let mut buttons = Vec::new();
        let mut actions = Vec::new();
        for rank in &donate_ranks {
            let owned = if viewer_profile.donate_rank == rank.id {
                " §a(Ваш)"
            } else {
                ""
            };
            buttons.push(SimpleFormButton {
                text: legacy(&format!(
                    "{}{}\n§e{} ₽ §8| §7Дома: §f{} §8| §7Приваты: §f{}",
                    rank.prefix.trim(),
                    owned,
                    rank.price_rub,
                    rank.perks.max_homes,
                    rank.perks.max_regions
                )),
                image: None,
            });
            actions.push(BedrockRankAction::ShowRankDetails(rank.id.clone()));
        }
        buttons.push(SimpleFormButton {
            text: legacy("§b✦ Отделы Стаффа (/staff)"),
            image: None,
        });
        actions.push(BedrockRankAction::OpenStaff);
        buttons.push(SimpleFormButton {
            text: legacy("§e★ Мой профиль (/rank info)"),
            image: None,
        });
        actions.push(BedrockRankAction::OpenProfile(viewer_name.clone()));

        let content = format!(
            "§7Ваш донат-ранг: §e{}\n§7Магазин сервера: §b{}\n§7Выберите привилегию для подробностей:",
            viewer_profile.donate_rank.to_uppercase(),
            config.store_url
        );
        open_bedrock_form(
            player,
            &viewer_name,
            "§6§l✦ ПРИВИЛЕГИИ KRONOS",
            &content,
            buttons,
            actions,
        );
    }

    // Java interactive clickable chat menu
    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§6§l✦ ДОНАТ-ПРИВИЛЕГИИ {} §8(10 ступеней)",
            config.server_name
        )),
        false,
    );

    let nav = legacy(" ")
        .add_child(clickable_run(
            "§e§l[★ Мой профиль]",
            "/rank info",
            "§eОткрыть вашу Dual-Track карточку рангов и лимитов",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            "§b§l[⚑ Команда Стаффа]",
            "/staff",
            "§bПосмотреть 9 должностей в 5 отделах и кто онлайн",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            "§7[Список 20 рангов]",
            "/rank list",
            "§7Показать полный список всех 20 рангов с весами",
        ));
    player.send_system_message(nav, false);

    for rank in &donate_ranks {
        let is_current = viewer_profile.donate_rank == rank.id;
        let mark = if is_current { "§a✔ " } else { "§6▸ " };
        let hover = build_rank_hover_tooltip(rank, config);

        let mut row = legacy(&format!("  {}", mark))
            .add_child(clickable_run(
                &format!("{} §f{}", rank.prefix.trim(), rank.display_name),
                &format!("/rank info {}", viewer_name),
                &hover,
            ))
            .add_child(legacy(&format!(
                " §8— §e{} ₽ §8| §7Дома: §a{} §8| §7Приваты: §b{}",
                rank.price_rub, rank.perks.max_homes, rank.perks.max_regions
            )));
        if is_current {
            row = row.add_child(legacy(" §a[АКТИВЕН]"));
        }
        player.send_system_message(row, false);
    }

    let footer = legacy(&format!(" §7Покупка: §b{} §8| ", config.store_url))
        .add_child(clickable_suggest(
            "§6[Установить суффикс]",
            "/rank suffix ",
            "§6Доступно с ранга §5[EMPEROR]\n§7Команда: §f/rank suffix <текст|clear>",
        ));
    player.send_system_message(footer, false);
    player.send_system_message(legacy(LINE_SEP), false);
}

/// Opens the interactive Staff Departments Hub (`/staff`) showing all 9 roles across 5 departments.
pub fn open_staff_departments(
    player: &Player,
    config: &RanksConfig,
    profiles: &HashMap<String, PlayerRankProfile>,
    online_keys: &HashSet<String>,
    can_hr: bool,
    can_audit: bool,
) {
    play_click_sound(player);
    let viewer_name = player.get_name();
    let staff_ranks = config.staff_ranks();

    // Group online staff by staff_rank id
    let mut online_by_rank: HashMap<String, Vec<String>> = HashMap::new();
    for prof in profiles.values() {
        if online_keys.contains(&prof.player_key) {
            if let Some(ref sr) = prof.staff_rank {
                online_by_rank
                    .entry(sr.clone())
                    .or_default()
                    .push(prof.player_name.clone());
            }
        }
    }

    // Bedrock native SimpleForm
    if player.as_bedrock().is_some() {
        let mut buttons = Vec::new();
        let mut actions = Vec::new();
        for rank in &staff_ranks {
            let cnt = online_by_rank.get(&rank.id).map(|v| v.len()).unwrap_or(0);
            buttons.push(SimpleFormButton {
                text: legacy(&format!(
                    "{} §7(Вес {})\n{} §8| §aОнлайн: {}",
                    rank.prefix.trim(),
                    rank.weight,
                    rank.department.label_ru(),
                    cnt
                )),
                image: None,
            });
            actions.push(BedrockRankAction::ShowRankDetails(rank.id.clone()));
        }
        if can_hr {
            buttons.push(SimpleFormButton {
                text: legacy("§d📋 HR-панель заявок (/staff hr)"),
                image: None,
            });
            actions.push(BedrockRankAction::OpenHr);
        }
        if can_audit {
            buttons.push(SimpleFormButton {
                text: legacy("§6📜 Журнал назначений (/staff audit)"),
                image: None,
            });
            actions.push(BedrockRankAction::OpenAudit);
        }
        buttons.push(SimpleFormButton {
            text: legacy("§6✦ Донат-привилегии (/donate)"),
            image: None,
        });
        actions.push(BedrockRankAction::OpenDonate);

        open_bedrock_form(
            player,
            &viewer_name,
            "§b§l⚑ ОТДЕЛЫ СТАФФА KRONOS",
            "§79 должностей в 5 независимых отделах.\n§7Выберите должность для просмотра полномочий:",
            buttons,
            actions,
        );
    }

    // Java interactive clickable chat menu
    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy("§b§l⚑ ИЕРАРХИЯ СТАФФА KRONOS §8(9 должностей · 5 отделов)"),
        false,
    );

    let mut bar = legacy(" ")
        .add_child(clickable_suggest(
            "§a§l[✉ Подать заявку]",
            "/staff apply trainee ",
            "§aПодать заявку в Отдел Набора (HR)\n§7Использование: §f/staff apply <должность> <о_себе>",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            "§e[Кто онлайн]",
            "/staff online",
            "§eПоказать всех сотрудников стаффа сейчас в сети",
        ));
    if can_hr {
        bar = bar
            .add_child(legacy(" "))
            .add_child(clickable_run(
                "§d§l[HR-панель]",
                "/staff hr",
                "§dОткрыть список заявок в стафф (приём/отказ в 1 клик)",
            ));
    }
    if can_audit {
        bar = bar
            .add_child(legacy(" "))
            .add_child(clickable_run(
                "§6[Аудит]",
                "/staff audit",
                "§6Открыть журнал последних назначений и снятий",
            ));
    }
    bar = bar
        .add_child(legacy(" "))
        .add_child(clickable_run(
            "§6[Донат]",
            "/donate",
            "§6Перейти к витрине 10 донат-привилегий",
        ));
    player.send_system_message(bar, false);

    for rank in &staff_ranks {
        let mut online_list = online_by_rank.get(&rank.id).cloned().unwrap_or_default();
        online_list.sort();
        let online_text = if online_list.is_empty() {
            "§8никого нет".to_string()
        } else {
            format!("§a{}", online_list.join("§7, §a"))
        };
        let hover = build_rank_hover_tooltip(rank, config);

        let row = legacy("  §8▸ ")
            .add_child(clickable_suggest(
                &format!("{} §7(вес §e{}§7)", rank.prefix.trim(), rank.weight),
                &format!("/staff apply {} ", rank.id),
                &hover,
            ))
            .add_child(legacy(&format!(
                " §8| {} §8| §7Онлайн: {}",
                rank.department.label_ru(),
                online_text
            )));
        player.send_system_message(row, false);
    }

    player.send_system_message(legacy(LINE_SEP), false);
}

/// Opens a Dual-Track Profile Card (`/rank info [игрок]`).
pub fn open_player_profile(
    player: &Player,
    config: &RanksConfig,
    target: &PlayerRankProfile,
    now_ms: u64,
) {
    play_click_sound(player);
    let primary = primary_rank(config, target);
    let donate_def = config
        .find(&target.donate_rank)
        .unwrap_or_else(|| config.default_rank());
    let staff_def = target.staff_rank.as_deref().and_then(|id| config.find(id));
    let perks = effective_perks(config, target);
    let suffix = resolved_suffix(config, target);

    let donate_exp = format_expiry(target.donate_expires_at_ms, now_ms);
    let staff_exp = format_expiry(target.staff_expires_at_ms, now_ms);

    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§6§l✦ ПРОФИЛЬ РАНГОВ §8— {}{}{}{}",
            primary.prefix, primary.name_color, target.player_name, suffix
        )),
        false,
    );
    player.send_system_message(
        legacy(&format!(
            " §7Основной статус: {} §8(вес §e{}§8, {})",
            primary.display_name,
            primary.weight,
            primary.department.label_ru()
        )),
        false,
    );
    player.send_system_message(
        legacy(&format!(
            " §6Донат-ветка: {} §f{} §8({})",
            donate_def.prefix.trim(),
            donate_def.display_name,
            donate_exp
        )),
        false,
    );
    match staff_def {
        Some(sd) => {
            player.send_system_message(
                legacy(&format!(
                    " §bСтафф-ветка: {} §f{} §8({}, {})",
                    sd.prefix.trim(),
                    sd.display_name,
                    sd.department.label_ru(),
                    staff_exp
                )),
                false,
            );
        }
        None => {
            player.send_system_message(legacy(" §bСтафф-ветка: §8нет должности"), false);
        }
    }
    player.send_system_message(
        legacy(&format!(
            " §7Итоговые лимиты (Dual-Track): §aДома: §f{} §8| §bПриваты: §f{} §7(до §f{}§7 бл.) §8| §eXP: §fx{:.2}",
            perks.max_homes, perks.max_regions, perks.max_region_volume, perks.xp_multiplier
        )),
        false,
    );
    player.send_system_message(
        legacy(&format!(
            " §7Телепорт: §f{}с §8| §7RTP КД: §f{}с §8| §7Цветной чат: {} §8| §7Полёт в привате: {}",
            perks.tp_warmup_seconds,
            perks.rtp_cooldown_seconds,
            bool_badge(perks.colored_chat),
            bool_badge(perks.fly_in_own_region),
        )),
        false,
    );
    if !target.extra_permissions.is_empty() {
        player.send_system_message(
            legacy(&format!(
                " §7Личные права (§e{}§7): §f{}",
                target.extra_permissions.len(),
                target.extra_permissions.join("§7, §f")
            )),
            false,
        );
    }

    let mut actions = legacy(" ")
        .add_child(clickable_run(
            "§6[Витрина доната]",
            "/donate",
            "§6Посмотреть все 10 донат-привилегий",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            "§b[Отделы стаффа]",
            "/staff",
            "§bПосмотреть иерархию отделов стаффа",
        ));
    if perks.custom_suffix {
        actions = actions
            .add_child(legacy(" "))
            .add_child(clickable_suggest(
                "§d[Сменить суффикс]",
                "/rank suffix ",
                "§dУстановить кастомный суффикс в чате и Табе",
            ));
    }
    player.send_system_message(actions, false);
    player.send_system_message(legacy(LINE_SEP), false);
}

/// Opens the interactive HR Panel (`/staff hr`) with 1-click approve & reject buttons.
pub fn open_hr_panel(player: &Player, applications: &[StaffApplication]) {
    play_click_sound(player);
    let viewer_name = player.get_name();
    let pending: Vec<&StaffApplication> = applications
        .iter()
        .filter(|a| a.status == "pending")
        .collect();

    if player.as_bedrock().is_some() {
        let mut buttons = Vec::new();
        let mut actions = Vec::new();
        for app in pending.iter().take(10) {
            let role_label = if app.desired_role.is_empty() {
                "trainee"
            } else {
                &app.desired_role
            };
            buttons.push(SimpleFormButton {
                text: legacy(&format!(
                    "§a✔ Принять #{} — {} ({})\n§7{}",
                    app.id, app.applicant_name, role_label, app.note
                )),
                image: None,
            });
            actions.push(BedrockRankAction::AcceptApplication(app.id));

            buttons.push(SimpleFormButton {
                text: legacy(&format!("§c✖ Отклонить #{} — {}", app.id, app.applicant_name)),
                image: None,
            });
            actions.push(BedrockRankAction::RejectApplication(app.id));
        }
        buttons.push(SimpleFormButton {
            text: legacy("§b← Назад в /staff"),
            image: None,
        });
        actions.push(BedrockRankAction::OpenStaff);

        open_bedrock_form(
            player,
            &viewer_name,
            "§d§l📋 HR-ПАНЕЛЬ ЗАЯВОК KRONOS",
            &format!("§7Ожидают рассмотрения: §e{}", pending.len()),
            buttons,
            actions,
        );
    }

    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§d§l📋 ОТДЕЛ НАБОРА (HR) §8— §7Активных заявок: §e{}",
            pending.len()
        )),
        false,
    );

    if pending.is_empty() {
        player.send_system_message(
            legacy(" §7Нет нерассмотренных заявок. Новые заявки подаются через §f/staff apply§7."),
            false,
        );
    } else {
        for app in pending.iter().take(12) {
            let role = if app.desired_role.is_empty() {
                "trainee"
            } else {
                &app.desired_role
            };
            let row = legacy(&format!(
                "  §d#{} §f{} §8(§e{}§8): §7{} ",
                app.id, app.applicant_name, role, app.note
            ))
            .add_child(clickable_run(
                "§a§l[✔ Принять]",
                &format!("/staff hr accept {}", app.id),
                &format!(
                    "§aОдобрить заявку #{} игрока §f{}\n§7Игрок будет назначен на §e{}",
                    app.id, app.applicant_name, role
                ),
            ))
            .add_child(legacy(" "))
            .add_child(clickable_run(
                "§c§l[✖ Отклонить]",
                &format!("/staff hr reject {}", app.id),
                &format!("§cОтклонить заявку #{} игрока §f{}", app.id, app.applicant_name),
            ));
            player.send_system_message(row, false);
        }
    }

    player.send_system_message(legacy(LINE_SEP), false);
}

/// Opens the recent Staff & Rank Audit Log (`/staff audit`).
pub fn open_audit_menu(player: &Player, audit_log: &[RankAuditEntry]) {
    play_click_sound(player);
    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§6§l📜 АУДИТ РАНГОВ KRONOS §8(последние из {})",
            audit_log.len()
        )),
        false,
    );

    if audit_log.is_empty() {
        player.send_system_message(legacy(" §7Журнал аудита пока пуст."), false);
    } else {
        for entry in audit_log.iter().rev().take(15) {
            let color = match entry.action.as_str() {
                "SET" | "PERM_ADD" => "§a",
                "RESET" | "REMOVE_STAFF" | "REMOVE_DONATE" | "PERM_REMOVE" => "§c",
                _ => "§e",
            };
            player.send_system_message(
                legacy(&format!(
                    "  §8#{} §f{} §8→ {}{} §f{} §8(§e{}§8, срок: §7{}§8)",
                    entry.id,
                    entry.actor_name,
                    color,
                    entry.action,
                    entry.target_name,
                    entry.rank_id,
                    entry.duration_label
                )),
                false,
            );
        }
    }

    player.send_system_message(legacy(LINE_SEP), false);
}

/// Sends a detailed rank card in chat (also triggered by Bedrock form button clicks).
pub fn send_rank_details_card(player: &Player, rank: &RankDefinition, config: &RanksConfig) {
    play_click_sound(player);
    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&build_rank_hover_tooltip(rank, config)),
        false,
    );
    player.send_system_message(legacy(LINE_SEP), false);
}

pub fn build_rank_hover_tooltip(rank: &RankDefinition, config: &RanksConfig) -> String {
    let price_line = if rank.price_rub > 0 {
        format!("\n§7Цена: §e{} ₽ §8({})", rank.price_rub, config.store_url)
    } else {
        format!("\n§7Отдел: {}", rank.department.label_ru())
    };
    let gov_line = if rank.max_assignable_weight > 0 {
        format!(
            "\n§7Назначает отделы: §b{} §7(до веса §e{}§7)",
            rank.assignable_departments.join(", "),
            rank.max_assignable_weight
        )
    } else {
        String::new()
    };

    format!(
        "{} §f§l{}§r §8(ID: {}, вес: {}){}{}\n§7Дома: §a{} §8| §7Приваты: §b{} §7(до {} бл.)\n§7Задержка TP: §f{}с §8| §7RTP КД: §f{}с §8| §7XP: §ex{:.2}\n§7Особенности: §f{}",
        rank.prefix.trim(),
        rank.display_name,
        rank.id,
        rank.weight,
        price_line,
        gov_line,
        rank.perks.max_homes,
        rank.perks.max_regions,
        rank.perks.max_region_volume,
        rank.perks.tp_warmup_seconds,
        rank.perks.rtp_cooldown_seconds,
        rank.perks.xp_multiplier,
        rank.perks.summary,
    )
}

fn format_expiry(expires_at_ms: Option<u64>, now_ms: u64) -> String {
    match expires_at_ms {
        None => "навсегда".into(),
        Some(exp) if exp <= now_ms => "истекает...".into(),
        Some(exp) => format!("осталось {}", format_duration_ru((exp - now_ms) / 1000)),
    }
}

fn bool_badge(v: bool) -> &'static str {
    if v {
        "§a✔"
    } else {
        "§c✖"
    }
}
