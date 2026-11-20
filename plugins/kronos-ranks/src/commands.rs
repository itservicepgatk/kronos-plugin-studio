use std::collections::HashSet;

use pumpkin_plugin_api::{
    command::{
        ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType,
    },
    commands::CommandHandler,
    wit::pumpkin::plugin::command::Arg,
    Context, Player, Server,
};

use crate::hierarchy::{
    assign_rank, can_assign_rank, effective_perks, has_permission, is_staff_member, normalize_key,
    primary_rank, push_audit, remove_rank_track, reset_all_ranks,
};
use crate::menu::{
    clickable_run, clickable_suggest, open_audit_menu, open_donate_showcase, open_hr_panel,
    open_player_profile, open_staff_departments, send_rank_details_card,
};
use crate::models::{
    format_duration_ru, parse_duration_seconds, BedrockRankAction, PlayerRankProfile,
    StaffApplication,
};
use crate::storage::{
    ensure_profile, get_or_default_profile, now_ms, save_current_store, with_state,
};
use crate::tab_chat::{
    find_online_player_ci, format_staff_chat_line, legacy, play_chime_sound, play_error_sound,
    play_rank_up_sound, play_success_sound, refresh_online_player_by_name,
};

pub const PERM_PLAYER: &str = "kronos-ranks:player";
pub const PERM_STAFF: &str = "kronos-ranks:staff";
pub const PERM_ADMIN: &str = "kronos-ranks:admin";

fn extract_args(args: &ConsumedArgs) -> String {
    match args.get_value("args") {
        Arg::Simple(s) | Arg::Msg(s) => s,
        _ => String::new(),
    }
}

fn require_player(sender: &CommandSender) -> Option<Player> {
    match sender.as_player() {
        Some(p) => Some(p),
        None => {
            sender.send_error(legacy("§cЭта команда доступна только игрокам!"));
            None
        }
    }
}

fn actor_is_privileged_console(sender: &CommandSender) -> bool {
    if sender.is_console() {
        return true;
    }
    if let Some(p) = sender.as_player() {
        return p.has_permission(PERM_ADMIN);
    }
    false
}

/// Checks if a player/sender can view or manage HR applications.
fn sender_can_hr(sender: &CommandSender) -> bool {
    if actor_is_privileged_console(sender) {
        return true;
    }
    let Some(player) = sender.as_player() else {
        return false;
    };
    let name = player.get_name();
    let now = now_ms();
    with_state(|st| {
        let prof = get_or_default_profile(&st.store, &name, now);
        has_permission(&st.config, &prof, "kronos.staff.hr.panel")
            || has_permission(&st.config, &prof, "kronos.ranks.hr")
            || primary_rank(&st.config, &prof).weight >= 420
    })
}

/// Checks if a player/sender can view the staff audit log.
fn sender_can_audit(sender: &CommandSender) -> bool {
    if actor_is_privileged_console(sender) {
        return true;
    }
    let Some(player) = sender.as_player() else {
        return false;
    };
    let name = player.get_name();
    let now = now_ms();
    with_state(|st| {
        let prof = get_or_default_profile(&st.store, &name, now);
        has_permission(&st.config, &prof, "kronos.staff.hr.audit")
            || has_permission(&st.config, &prof, "kronos.ranks.audit")
            || primary_rank(&st.config, &prof).weight >= 300
    })
}

/// Handles Bedrock `SimpleForm` button presses dispatched from `BedrockFormResponseEvent`.
pub fn execute_bedrock_action(player: &Player, server: &Server, action: BedrockRankAction) {
    let p_name = player.get_name();
    let now = now_ms();

    match action {
        BedrockRankAction::OpenDonate => {
            let (config, profile) = with_state(|st| {
                let prof = get_or_default_profile(&st.store, &p_name, now);
                (st.config.clone(), prof)
            });
            open_donate_showcase(player, &config, &profile);
        }
        BedrockRankAction::OpenStaff => {
            let online_keys: HashSet<String> = server
                .get_all_players()
                .into_iter()
                .map(|p| normalize_key(&p.get_name()))
                .collect();
            let is_admin = player.has_permission(PERM_ADMIN);
            let (config, profiles, can_hr, can_audit) = with_state(|st| {
                let prof = get_or_default_profile(&st.store, &p_name, now);
                let hr = is_admin
                    || has_permission(&st.config, &prof, "kronos.staff.hr.panel")
                    || has_permission(&st.config, &prof, "kronos.ranks.hr")
                    || primary_rank(&st.config, &prof).weight >= 420;
                let audit = is_admin
                    || has_permission(&st.config, &prof, "kronos.staff.hr.audit")
                    || has_permission(&st.config, &prof, "kronos.ranks.audit")
                    || primary_rank(&st.config, &prof).weight >= 300;
                (st.config.clone(), st.store.profiles.clone(), hr, audit)
            });
            open_staff_departments(player, &config, &profiles, &online_keys, can_hr, can_audit);
        }
        BedrockRankAction::OpenProfile(target_name) => {
            let (config, profile) = with_state(|st| {
                let prof = get_or_default_profile(&st.store, &target_name, now);
                (st.config.clone(), prof)
            });
            open_player_profile(player, &config, &profile, now);
        }
        BedrockRankAction::OpenHr => {
            let apps = with_state(|st| st.store.applications.clone());
            open_hr_panel(player, &apps);
        }
        BedrockRankAction::OpenAudit => {
            let audit = with_state(|st| st.store.audit_log.clone());
            open_audit_menu(player, &audit);
        }
        BedrockRankAction::ShowRankDetails(rank_id) => {
            let (config, rank_opt) =
                with_state(|st| (st.config.clone(), st.config.find(&rank_id).cloned()));
            if let Some(rank) = rank_opt {
                send_rank_details_card(player, &rank, &config);
            }
        }
        BedrockRankAction::AcceptApplication(app_id) => {
            let is_admin = player.has_permission(PERM_ADMIN);
            match review_application_by_actor(
                server,
                Some(&normalize_key(&p_name)),
                &p_name,
                is_admin,
                app_id,
                true,
            ) {
                Ok(msg) => {
                    play_success_sound(player);
                    player.send_system_message(legacy(&msg), false);
                }
                Err(err) => {
                    play_error_sound(player);
                    player.send_system_message(legacy(&err), false);
                }
            }
        }
        BedrockRankAction::RejectApplication(app_id) => {
            let is_admin = player.has_permission(PERM_ADMIN);
            match review_application_by_actor(
                server,
                Some(&normalize_key(&p_name)),
                &p_name,
                is_admin,
                app_id,
                false,
            ) {
                Ok(msg) => {
                    play_success_sound(player);
                    player.send_system_message(legacy(&msg), false);
                }
                Err(err) => {
                    play_error_sound(player);
                    player.send_system_message(legacy(&err), false);
                }
            }
        }
    }
}

/// Core HR application approval/rejection logic shared by `/staff hr accept|reject <id>` and Bedrock HR form.
fn review_application_by_actor(
    server: &Server,
    actor_key: Option<&str>,
    actor_name: &str,
    actor_privileged: bool,
    app_id: u64,
    accept: bool,
) -> Result<String, String> {
    let now = now_ms();
    let outcome: Result<(String, Option<String>, Option<String>), String> = with_state(|st| {
        if !actor_privileged {
            let Some(k) = actor_key else {
                return Err("§c✗ Недостаточно прав для работы с HR-панелью.".into());
            };
            let prof = st
                .store
                .profiles
                .get(k)
                .cloned()
                .unwrap_or_else(|| PlayerRankProfile::new(actor_name, now));
            if !has_permission(&st.config, &prof, "kronos.staff.hr.panel")
                && !has_permission(&st.config, &prof, "kronos.ranks.hr")
                && primary_rank(&st.config, &prof).weight < 420
            {
                return Err("§c✗ Одобрять/отклонять заявки может только Отдел Набора (HR) и выше.".into());
            }
        }

        let Some(idx) = st.store.applications.iter().position(|a| a.id == app_id) else {
            return Err(format!("§c✗ Заявка §e#{} §cне найдена.", app_id));
        };
        if st.store.applications[idx].status != "pending" {
            return Err(format!(
                "§c✗ Заявка §e#{} §cуже обработана (статус: §f{}§c).",
                app_id, st.store.applications[idx].status
            ));
        }

        let applicant_name = st.store.applications[idx].applicant_name.clone();
        let desired_role = st.store.applications[idx].desired_role.clone();

        if !accept {
            st.store.applications[idx].status = "rejected".into();
            st.store.applications[idx].reviewed_by = Some(actor_name.to_string());
            st.dirty = true;
            return Ok((applicant_name, None, None));
        }

        // Determine target staff role: try desired_role if valid staff rank & actor can assign it, else trainee
        let chosen_rank_id = if !desired_role.is_empty() {
            if let Some(def) = st.config.find(&desired_role).cloned() {
                let actor_prof = actor_key.and_then(|k| st.store.profiles.get(k));
                let target_prof = st.store.profiles.get(&normalize_key(&applicant_name));
                if def.weight >= 100
                    && can_assign_rank(&st.config, actor_prof, actor_privileged, target_prof, &def)
                        .is_ok()
                {
                    def.id
                } else {
                    "trainee".to_string()
                }
            } else {
                "trainee".to_string()
            }
        } else {
            "trainee".to_string()
        };

        let (_, assigned_def) = assign_rank(
            &st.config,
            &mut st.store,
            &mut st.dirty,
            actor_key,
            actor_name,
            actor_privileged,
            &applicant_name,
            &chosen_rank_id,
            None,
            now,
        )?;

        st.store.applications[idx].status = "accepted".into();
        st.store.applications[idx].reviewed_by = Some(actor_name.to_string());
        Ok((
            applicant_name,
            Some(assigned_def.prefix),
            Some(assigned_def.display_name),
        ))
    });

    let (applicant_name, assigned_prefix, assigned_display) = outcome?;
    save_current_store();

    if accept {
        refresh_online_player_by_name(server, &applicant_name);
        let prefix = assigned_prefix.unwrap_or_else(|| "§a[Стажёр] ".into());
        let display = assigned_display.unwrap_or_else(|| "Стажёр".into());

        if let Some(target_player) = find_online_player_ci(server, &applicant_name) {
            play_rank_up_sound(&target_player);
            target_player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS HR §8» §aПоздравляем! Ваша заявка §e#{} §aодобрена сотрудником §f{}§a! Вы назначены на должность {}§f{}§a.",
                    app_id, actor_name, prefix, display
                )),
                false,
            );
        }

        Ok(format!(
            "§6§l✦ KRONOS HR §8» §aЗаявка §e#{} §aодобрена! Игрок §f{} §aполучил должность {}§f{}§a.",
            app_id, applicant_name, prefix, display
        ))
    } else {
        if let Some(target_player) = find_online_player_ci(server, &applicant_name) {
            play_error_sound(&target_player);
            target_player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS HR §8» §cВаша заявка §e#{} §cбыла отклонена сотрудником §f{}§c.",
                    app_id, actor_name
                )),
                false,
            );
        }

        Ok(format!(
            "§6§l✦ KRONOS HR §8» §eЗаявка §f#{} §eигрока §f{} §eотклонена.",
            app_id, applicant_name
        ))
    }
}

// =============================================================================
// /donate & /ranks
// =============================================================================

struct DonateCmd;
impl CommandHandler for DonateCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = require_player(&sender) else {
            return Ok(0);
        };
        let p_name = player.get_name();
        let now = now_ms();
        let (config, profile) = with_state(|st| {
            let prof = get_or_default_profile(&st.store, &p_name, now);
            (st.config.clone(), prof)
        });
        open_donate_showcase(&player, &config, &profile);
        Ok(0)
    }
}

// =============================================================================
// /staff [online | apply <должность> <о_себе> | hr [accept|reject <id>] | audit]
// =============================================================================

struct StaffCmd;
impl CommandHandler for StaffCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let raw = extract_args(&args);
        let parts: Vec<&str> = raw.split_whitespace().collect();

        if parts.is_empty() {
            let Some(player) = require_player(&sender) else {
                return Ok(0);
            };
            let online_keys: HashSet<String> = server
                .get_all_players()
                .into_iter()
                .map(|p| normalize_key(&p.get_name()))
                .collect();
            let can_hr = sender_can_hr(&sender);
            let can_audit = sender_can_audit(&sender);
            let (config, profiles) =
                with_state(|st| (st.config.clone(), st.store.profiles.clone()));
            open_staff_departments(&player, &config, &profiles, &online_keys, can_hr, can_audit);
            return Ok(0);
        }

        match parts[0].to_lowercase().as_str() {
            "online" => {
                let online_players = server.get_all_players();
                let online_names: Vec<String> =
                    online_players.iter().map(|p| p.get_name()).collect();
                let now = now_ms();
                let lines = with_state(|st| {
                    let mut found: Vec<(u32, String)> = Vec::new();
                    for p_name in &online_names {
                        let prof = get_or_default_profile(&st.store, p_name, now);
                        if is_staff_member(&st.config, &prof) {
                            let primary = primary_rank(&st.config, &prof);
                            found.push((
                                primary.weight,
                                format!(
                                    "  {}{} §8— {}",
                                    primary.prefix,
                                    prof.player_name,
                                    primary.department.label_ru()
                                ),
                            ));
                        }
                    }
                    found.sort_by(|a, b| b.0.cmp(&a.0));
                    found
                });

                sender.send_system_message(legacy(
                    "§8§m────────────────────────────────────────────",
                ));
                sender.send_system_message(legacy(&format!(
                    "§b§l⚑ СТАФФ ОНЛАЙН §7(Всего: §a{}§7)",
                    lines.len()
                )));
                if lines.is_empty() {
                    sender.send_system_message(legacy(" §7Сейчас нет сотрудников стаффа в сети."));
                } else {
                    for (_, line) in lines {
                        sender.send_system_message(legacy(&line));
                    }
                }
                sender.send_system_message(legacy(
                    "§8§m────────────────────────────────────────────",
                ));
            }
            "apply" => {
                let Some(player) = require_player(&sender) else {
                    return Ok(0);
                };
                if parts.len() < 2 {
                    play_error_sound(&player);
                    sender.send_error(legacy(
                        "§cИспользование: §f/staff apply <должность> <о_себе> §7(например: /staff apply trainee Мне 17, хочу помогать игрокам)",
                    ));
                    return Ok(0);
                }

                let p_name = player.get_name();
                let p_key = normalize_key(&p_name);
                let now = now_ms();

                // Check if parts[1] matches a known staff rank id; otherwise treat entire text as bio for trainee
                let (desired_role, note) = with_state(|st| {
                    if let Some(def) = st.config.find(parts[1]) {
                        if def.weight >= 100 && parts.len() >= 3 {
                            return (def.id.clone(), parts[2..].join(" "));
                        }
                    }
                    ("trainee".to_string(), parts[1..].join(" "))
                });

                if note.chars().count() < 5 {
                    play_error_sound(&player);
                    sender.send_error(legacy(
                        "§cРасскажите о себе подробнее (минимум 5 символов).",
                    ));
                    return Ok(0);
                }

                let created: Result<u64, String> = with_state(|st| {
                    let already_pending = st
                        .store
                        .applications
                        .iter()
                        .any(|a| a.applicant_key == p_key && a.status == "pending");
                    if already_pending {
                        return Err(
                            "§c✗ У вас уже есть активная заявка в Отдел Набора (HR). Ожидайте рассмотрения."
                                .into(),
                        );
                    }
                    let id = st.store.next_app_id.max(1);
                    st.store.next_app_id = id + 1;
                    st.store.applications.push(StaffApplication {
                        id,
                        applicant_key: p_key.clone(),
                        applicant_name: p_name.clone(),
                        desired_role: desired_role.clone(),
                        note: note.clone(),
                        created_at_ms: now,
                        status: "pending".into(),
                        reviewed_by: None,
                    });
                    st.dirty = true;
                    Ok(id)
                });

                match created {
                    Ok(app_id) => {
                        save_current_store();
                        play_success_sound(&player);
                        sender.send_system_message(legacy(&format!(
                            "§6§l✦ KRONOS HR §8» §aЗаявка §e#{} §aна должность §e{} §aотправлена в Отдел Набора!",
                            app_id, desired_role
                        )));

                        // Notify online HR & leadership staff
                        let online_players = server.get_all_players();
                        let hr_keys: HashSet<String> = with_state(|st| {
                            st.store
                                .profiles
                                .values()
                                .filter(|prof| {
                                    has_permission(&st.config, prof, "kronos.staff.hr.panel")
                                        || has_permission(&st.config, prof, "kronos.ranks.hr")
                                        || primary_rank(&st.config, prof).weight >= 420
                                })
                                .map(|prof| prof.player_key.clone())
                                .collect()
                        });

                        for recipient in &online_players {
                            let r_key = normalize_key(&recipient.get_name());
                            if hr_keys.contains(&r_key) || recipient.has_permission(PERM_ADMIN) {
                                play_chime_sound(recipient);
                                let alert = legacy(&format!(
                                    "§d§l✦ НОВАЯ ЗАЯВКА #{} §8» §f{} §7(§e{}§7): §f{} ",
                                    app_id, p_name, desired_role, note
                                )).add_child(clickable_run(
                                    "§a§l[Открыть HR]",
                                    "/staff hr",
                                    "§dОткрыть HR-панель заявок",
                                ));
                                recipient.send_system_message(alert, false);
                            }
                        }
                    }
                    Err(err) => {
                        play_error_sound(&player);
                        sender.send_error(legacy(&err));
                    }
                }
            }
            "hr" => {
                if !sender_can_hr(&sender) {
                    if let Some(p) = sender.as_player() {
                        play_error_sound(&p);
                    }
                    sender.send_error(legacy(
                        "§c✗ HR-панель доступна только Отделу Набора (HR), Гл.Куратору и Владельцу.",
                    ));
                    return Ok(0);
                }

                if parts.len() >= 3 {
                    let action = parts[1].to_lowercase();
                    let Ok(app_id) = parts[2].parse::<u64>() else {
                        sender.send_error(legacy("§cУкажите корректный номер заявки (#ID)."));
                        return Ok(0);
                    };
                    let accept = match action.as_str() {
                        "accept" | "approve" | "принять" => true,
                        "reject" | "deny" | "отклонить" => false,
                        _ => {
                            sender.send_error(legacy(
                                "§cИспользование: §f/staff hr <accept|reject> <id>",
                            ));
                            return Ok(0);
                        }
                    };
                    let actor_name = sender.get_name();
                    let actor_key = sender.as_player().map(|p| normalize_key(&p.get_name()));
                    let privileged = actor_is_privileged_console(&sender);

                    match review_application_by_actor(
                        &server,
                        actor_key.as_deref(),
                        &actor_name,
                        privileged,
                        app_id,
                        accept,
                    ) {
                        Ok(msg) => {
                            if let Some(p) = sender.as_player() {
                                play_success_sound(&p);
                            }
                            sender.send_system_message(legacy(&msg));
                        }
                        Err(err) => {
                            if let Some(p) = sender.as_player() {
                                play_error_sound(&p);
                            }
                            sender.send_error(legacy(&err));
                        }
                    }
                    return Ok(0);
                }

                let apps = with_state(|st| st.store.applications.clone());
                if let Some(player) = sender.as_player() {
                    open_hr_panel(&player, &apps);
                } else {
                    let pending: Vec<&StaffApplication> =
                        apps.iter().filter(|a| a.status == "pending").collect();
                    sender.send_system_message(legacy(&format!(
                        "§dHR-заявки (ожидают: {}):",
                        pending.len()
                    )));
                    for app in pending {
                        sender.send_system_message(legacy(&format!(
                            "  #{} {} ({}): {}",
                            app.id, app.applicant_name, app.desired_role, app.note
                        )));
                    }
                }
            }
            "audit" => {
                if !sender_can_audit(&sender) {
                    if let Some(p) = sender.as_player() {
                        play_error_sound(&p);
                    }
                    sender.send_error(legacy(
                        "§c✗ Журнал аудита доступен со звания §c[Ст.Модер] §cи выше.",
                    ));
                    return Ok(0);
                }
                let audit = with_state(|st| st.store.audit_log.clone());
                if let Some(player) = sender.as_player() {
                    open_audit_menu(&player, &audit);
                } else {
                    for entry in audit.iter().rev().take(15) {
                        sender.send_system_message(legacy(&format!(
                            "#{} {} -> {} {} ({}, {})",
                            entry.id,
                            entry.actor_name,
                            entry.action,
                            entry.target_name,
                            entry.rank_id,
                            entry.duration_label
                        )));
                    }
                }
            }
            _ => {
                sender.send_error(legacy(
                    "§cИспользование: §f/staff [online | apply <должность> <о_себе> | hr | audit]",
                ));
            }
        }

        Ok(0)
    }
}

// =============================================================================
// /sc & /staffchat <сообщение>
// =============================================================================

struct StaffChatCmd;
impl CommandHandler for StaffChatCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let message = extract_args(&args).trim().to_string();
        if message.is_empty() {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy("§cИспользование: §f/sc <сообщение>"));
            return Ok(0);
        }

        let now = now_ms();
        let is_console = sender.is_console();
        let formatted_line = if is_console {
            format!("§8[§b§lSC§8] §4§l[CONSOLE] §8» §f{}", message)
        } else {
            let Some(player) = sender.as_player() else {
                return Ok(0);
            };
            let p_name = player.get_name();
            let has_perm = player.has_permission(PERM_STAFF) || player.has_permission(PERM_ADMIN);

            let (allowed, line) = with_state(|st| {
                let prof = get_or_default_profile(&st.store, &p_name, now);
                let ok = has_perm || is_staff_member(&st.config, &prof);
                let formatted = format_staff_chat_line(&st.config, &prof, &message);
                (ok, formatted)
            });

            if !allowed {
                play_error_sound(&player);
                sender.send_error(legacy(
                    "§c✗ Стафф-чат (/sc) доступен только сотрудникам со звания §a[Стажёр] §cи выше.",
                ));
                return Ok(0);
            }
            line
        };

        // Deliver ONLY to online players with `kronos-ranks:staff` permission OR rank weight >= 100
        let online_players = server.get_all_players();
        let (config, profiles) = with_state(|st| (st.config.clone(), st.store.profiles.clone()));

        for recipient in &online_players {
            let r_name = recipient.get_name();
            let r_prof = profiles
                .get(&normalize_key(&r_name))
                .cloned()
                .unwrap_or_else(|| PlayerRankProfile::new(&r_name, now));

            let can_receive = recipient.has_permission(PERM_STAFF)
                || recipient.has_permission(PERM_ADMIN)
                || is_staff_member(&config, &r_prof);

            if can_receive {
                recipient.send_system_message(legacy(&formatted_line), false);
            }
        }

        if is_console {
            sender.send_system_message(legacy(&formatted_line));
        }
        tracing::info!("[StaffChat] {}", formatted_line);
        Ok(0)
    }
}

// =============================================================================
// /rank & /setrank
// =============================================================================

struct RankCmd {
    /// When invoked as `/setrank <игрок> <ранг> [срок]`, treat args directly as `set ...`.
    direct_set: bool,
}

impl CommandHandler for RankCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let raw = extract_args(&args);
        let mut parts: Vec<&str> = raw.split_whitespace().collect();

        if self.direct_set {
            parts.insert(0, "set");
        }

        if parts.is_empty() {
            if let Some(player) = sender.as_player() {
                let p_name = player.get_name();
                let now = now_ms();
                let (config, prof) = with_state(|st| {
                    (
                        st.config.clone(),
                        get_or_default_profile(&st.store, &p_name, now),
                    )
                });
                open_player_profile(&player, &config, &prof, now);
            } else {
                send_rank_usage(&sender);
            }
            return Ok(0);
        }

        match parts[0].to_lowercase().as_str() {
            "info" | "profile" => handle_rank_info(&sender, parts.get(1).copied()),
            "list" => handle_rank_list(&sender),
            "set" => handle_rank_set(&sender, &server, &parts[1..]),
            "reset" => handle_rank_reset(&sender, &server, &parts[1..]),
            "remove" | "demote" => handle_rank_remove(&sender, &server, &parts[1..]),
            "suffix" => handle_rank_suffix(&sender, &server, &parts[1..]),
            "perm" | "permission" => handle_rank_perm(&sender, &server, &parts[1..]),
            _ => send_rank_usage(&sender),
        }

        Ok(0)
    }
}

fn send_rank_usage(sender: &CommandSender) {
    sender.send_system_message(legacy(
        "§6§l✦ KRONOS RANKS §8— §fКоманды управления:",
    ));
    sender.send_system_message(legacy(
        "  §e/rank info [игрок] §8— §7карточка Dual-Track профиля",
    ));
    sender.send_system_message(legacy(
        "  §e/rank list §8— §7список всех 20 рангов (донат и стафф)",
    ));
    sender.send_system_message(legacy(
        "  §e/rank set <игрок> <ранг> [срок|perm] §8— §7выдать ранг",
    ));
    sender.send_system_message(legacy(
        "  §e/rank remove <игрок> [staff|donate] §8— §7снять одну ветку",
    ));
    sender.send_system_message(legacy(
        "  §e/rank reset <игрок> §8— §7сбросить все ранги игрока",
    ));
    sender.send_system_message(legacy(
        "  §e/rank suffix <текст|clear> §8— §7кастомный суффикс (EMPEROR+)",
    ));
    sender.send_system_message(legacy(
        "  §e/rank perm <игрок> <add|remove|check> <право> §8— §7личные права",
    ));
}

fn handle_rank_info(sender: &CommandSender, target_opt: Option<&str>) {
    let now = now_ms();
    let target_name = match target_opt {
        Some(t) => t.to_string(),
        None => match sender.as_player() {
            Some(p) => p.get_name(),
            None => {
                sender.send_error(legacy("§cУкажите игрока: §f/rank info <игрок>"));
                return;
            }
        },
    };

    let (config, profile) = with_state(|st| {
        let prof = get_or_default_profile(&st.store, &target_name, now);
        (st.config.clone(), prof)
    });

    if let Some(player) = sender.as_player() {
        open_player_profile(&player, &config, &profile, now);
    } else {
        let primary = primary_rank(&config, &profile);
        let perks = effective_perks(&config, &profile);
        sender.send_system_message(legacy(&format!(
            "§6Профиль {}: primary={} (weight {}), donate={}, staff={:?}, homes={}, regions={}",
            profile.player_name,
            primary.id,
            primary.weight,
            profile.donate_rank,
            profile.staff_rank,
            perks.max_homes,
            perks.max_regions
        )));
    }
}

fn handle_rank_list(sender: &CommandSender) {
    let config = with_state(|st| st.config.clone());
    sender.send_system_message(legacy(
        "§8§m────────────────────────────────────────────",
    ));
    sender.send_system_message(legacy(
        "§6§l✦ ВСЕ РАНГИ KRONOS §8(1 Игрок + 10 Донат + 9 Стафф)",
    ));

    // Donate row
    let mut donate_line = legacy(" §6Донат: ");
    for (idx, r) in config.donate_ranks().iter().enumerate() {
        if idx > 0 {
            donate_line = donate_line.add_child(legacy("§8, "));
        }
        donate_line = donate_line.add_child(clickable_suggest(
            &format!("{}§7({})", r.prefix.trim(), r.id),
            &format!("/rank set <ник> {}", r.id),
            &format!(
                "{} §f{}\n§7ID: §e{} §8| §7Вес: §e{} §8| §7Цена: §e{} ₽\n§7Дома: §a{} §8| §7Приваты: §b{}",
                r.prefix.trim(),
                r.display_name,
                r.id,
                r.weight,
                r.price_rub,
                r.perks.max_homes,
                r.perks.max_regions
            ),
        ));
    }
    sender.send_system_message(donate_line);

    // Staff row
    let mut staff_line = legacy(" §bСтафф: ");
    for (idx, r) in config.staff_ranks().iter().enumerate() {
        if idx > 0 {
            staff_line = staff_line.add_child(legacy("§8, "));
        }
        staff_line = staff_line.add_child(clickable_suggest(
            &format!("{}§7({})", r.prefix.trim(), r.id),
            &format!("/rank set <ник> {}", r.id),
            &format!(
                "{} §f{}\n§7ID: §e{} §8| §7Вес: §e{}\n§7Отдел: {}",
                r.prefix.trim(),
                r.display_name,
                r.id,
                r.weight,
                r.department.label_ru()
            ),
        ));
    }
    sender.send_system_message(staff_line);
    sender.send_system_message(legacy(
        "§8§m────────────────────────────────────────────",
    ));
}

fn handle_rank_set(sender: &CommandSender, server: &Server, args: &[&str]) {
    if args.len() < 2 {
        if let Some(p) = sender.as_player() {
            play_error_sound(&p);
        }
        sender.send_error(legacy(
            "§cИспользование: §f/rank set <игрок> <ранг> [срок|perm] §7(например: /rank set Steve vip 30d)",
        ));
        return;
    }

    let target_name = args[0];
    let rank_id = args[1];
    let duration_secs = if let Some(raw_dur) = args.get(2) {
        match parse_duration_seconds(raw_dur) {
            Ok(opt) => opt,
            Err(()) => {
                if let Some(p) = sender.as_player() {
                    play_error_sound(&p);
                }
                sender.send_error(legacy(
                    "§cНекорректный срок! Примеры: §f30d§c, §f12ч§c, §f2w§c, §f1mo§c, §fperm",
                ));
                return;
            }
        }
    } else {
        None
    };

    let actor_name = sender.get_name();
    let actor_key = sender.as_player().map(|p| normalize_key(&p.get_name()));
    let actor_privileged = actor_is_privileged_console(sender);
    let now = now_ms();

    let result = with_state(|st| {
        assign_rank(
            &st.config,
            &mut st.store,
            &mut st.dirty,
            actor_key.as_deref(),
            &actor_name,
            actor_privileged,
            target_name,
            rank_id,
            duration_secs,
            now,
        )
    });

    match result {
        Ok((_profile, assigned)) => {
            save_current_store();
            refresh_online_player_by_name(server, target_name);

            let dur_text = match duration_secs {
                None => "навсегда".into(),
                Some(s) => format_duration_ru(s),
            };
            if let Some(p) = sender.as_player() {
                play_success_sound(&p);
            }
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS RANKS §8» §aИгроку §f{} §aвыдан ранг {}§f{} §8(срок: §e{}§8).",
                target_name, assigned.prefix, assigned.display_name, dur_text
            )));

            if let Some(target_player) = find_online_player_ci(server, target_name) {
                play_rank_up_sound(&target_player);
                target_player.show_title(legacy(&format!("{}§r", assigned.prefix.trim())));
                target_player.show_subtitle(legacy(&format!(
                    "§aВам выдан ранг §f{} §8({})",
                    assigned.display_name, dur_text
                )));
                target_player.send_system_message(
                    legacy(&format!(
                        "§6§l✦ KRONOS RANKS §8» §aВам выдан ранг {}§f{} §7(срок: §e{}§7) от §f{}§a!",
                        assigned.prefix, assigned.display_name, dur_text, actor_name
                    )),
                    false,
                );
            }
        }
        Err(err) => {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(&err));
        }
    }
}

fn handle_rank_reset(sender: &CommandSender, server: &Server, args: &[&str]) {
    if args.is_empty() {
        sender.send_error(legacy("§cИспользование: §f/rank reset <игрок>"));
        return;
    }

    let target_name = args[0];
    let actor_name = sender.get_name();
    let actor_key = sender.as_player().map(|p| normalize_key(&p.get_name()));
    let actor_privileged = actor_is_privileged_console(sender);
    let now = now_ms();

    let result = with_state(|st| {
        reset_all_ranks(
            &st.config,
            &mut st.store,
            &mut st.dirty,
            actor_key.as_deref(),
            &actor_name,
            actor_privileged,
            target_name,
            now,
        )
    });

    match result {
        Ok(_) => {
            save_current_store();
            refresh_online_player_by_name(server, target_name);
            if let Some(p) = sender.as_player() {
                play_success_sound(&p);
            }
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS RANKS §8» §eВсе ранги игрока §f{} §eсброшены до §7[Игрок]§e.",
                target_name
            )));
            if let Some(target_player) = find_online_player_ci(server, target_name) {
                target_player.send_system_message(
                    legacy("§6§l✦ KRONOS RANKS §8» §eВаши ранги были сброшены до §7[Игрок]§e."),
                    false,
                );
            }
        }
        Err(err) => {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(&err));
        }
    }
}

fn handle_rank_remove(sender: &CommandSender, server: &Server, args: &[&str]) {
    if args.is_empty() {
        sender.send_error(legacy(
            "§cИспользование: §f/rank remove <игрок> [staff|donate]",
        ));
        return;
    }
    let target_name = args[0];
    let track = args.get(1).copied().unwrap_or("staff");

    let actor_name = sender.get_name();
    let actor_key = sender.as_player().map(|p| normalize_key(&p.get_name()));
    let actor_privileged = actor_is_privileged_console(sender);
    let now = now_ms();

    let result = with_state(|st| {
        remove_rank_track(
            &st.config,
            &mut st.store,
            &mut st.dirty,
            actor_key.as_deref(),
            &actor_name,
            actor_privileged,
            target_name,
            track,
            now,
        )
    });

    match result {
        Ok(_) => {
            save_current_store();
            refresh_online_player_by_name(server, target_name);
            if let Some(p) = sender.as_player() {
                play_success_sound(&p);
            }
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS RANKS §8» §eУ игрока §f{} §eснята ветка §f{}§e.",
                target_name, track
            )));
            if let Some(target_player) = find_online_player_ci(server, target_name) {
                target_player.send_system_message(
                    legacy(&format!(
                        "§6§l✦ KRONOS RANKS §8» §eС вас снят ранг ветки §f{}§e.",
                        track
                    )),
                    false,
                );
            }
        }
        Err(err) => {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(&err));
        }
    }
}

fn handle_rank_suffix(sender: &CommandSender, server: &Server, args: &[&str]) {
    let Some(player) = require_player(sender) else {
        return;
    };
    if args.is_empty() {
        play_error_sound(&player);
        sender.send_error(legacy(
            "§cИспользование: §f/rank suffix <суффикс|clear>",
        ));
        return;
    }

    let p_name = player.get_name();
    let now = now_ms();
    let raw_suffix = args.join(" ");
    let is_clear = raw_suffix.eq_ignore_ascii_case("clear") || raw_suffix.eq_ignore_ascii_case("off");

    let update_res: Result<String, String> = with_state(|st| {
        let prof = ensure_profile(&mut st.store, &p_name, now);
        let perks = effective_perks(&st.config, prof);
        if !perks.custom_suffix && !has_permission(&st.config, prof, "kronos.ranks.suffix") {
            return Err(
                "§c✗ Кастомный суффикс доступен начиная с ранга §5[EMPEROR] §cили выше.".into(),
            );
        }

        if is_clear {
            prof.custom_suffix = None;
            prof.updated_at_ms = now;
            st.dirty = true;
            return Ok("§6§l✦ KRONOS RANKS §8» §7Ваш кастомный суффикс очищен.".into());
        }

        let formatted = raw_suffix.replace('§', "").replace('&', "§");
        let visible_len = strip_legacy_codes(&formatted).chars().count();
        if visible_len == 0 || visible_len > 16 {
            return Err("§c✗ Длина видимого текста суффикса должна быть от 1 до 16 символов.".into());
        }

        prof.custom_suffix = Some(formatted.clone());
        prof.updated_at_ms = now;
        st.dirty = true;
        Ok(format!(
            "§6§l✦ KRONOS RANKS §8» §aУстановлен суффикс: {}§r",
            formatted
        ))
    });

    match update_res {
        Ok(msg) => {
            save_current_store();
            refresh_online_player_by_name(server, &p_name);
            play_success_sound(&player);
            sender.send_system_message(legacy(&msg));
        }
        Err(err) => {
            play_error_sound(&player);
            sender.send_error(legacy(&err));
        }
    }
}

fn handle_rank_perm(sender: &CommandSender, server: &Server, args: &[&str]) {
    if args.len() < 3 {
        sender.send_error(legacy(
            "§cИспользование: §f/rank perm <игрок> <add|remove|check> <право>",
        ));
        return;
    }

    let target_name = args[0];
    let action = args[1].to_lowercase();
    let node = args[2].trim();

    let actor_name = sender.get_name();
    let actor_key = sender.as_player().map(|p| normalize_key(&p.get_name()));
    let is_console = sender.is_console();
    let has_admin = actor_is_privileged_console(sender);
    let now = now_ms();

    let res: Result<(String, bool), String> = with_state(|st| {
        let actor_prof = actor_key.and_then(|k| st.store.profiles.get(&k)).cloned();
        let actor_is_owner = is_console
            || actor_prof
                .as_ref()
                .map(|p| primary_rank(&st.config, p).id == "owner" || has_permission(&st.config, p, "*"))
                .unwrap_or(false);

        let can_manage_perms = actor_is_owner
            || has_admin
            || actor_prof
                .as_ref()
                .map(|p| {
                    has_permission(&st.config, p, "kronos.ranks.perm.manage")
                        || has_permission(&st.config, p, "kronos.ranks.perm")
                })
                .unwrap_or(false);

        if !can_manage_perms {
            return Err("§c✗ Управление правами доступно Тех.Админу, Гл.Куратору и Владельцу.".into());
        }

        // Critical security guard: only Owner or Console may grant/revoke wildcard or kronos.ranks.* nodes
        let lower_node = node.to_lowercase();
        let is_restricted_node = lower_node == "*"
            || lower_node == "kronos.*"
            || lower_node.starts_with("kronos.ranks.")
            || lower_node == "kronos.ranks";

        if matches!(action.as_str(), "add" | "remove") && is_restricted_node && !actor_is_owner {
            return Err(
                "§c✗ Безопасность: права §f*§c, §fkronos.* §cи §fkronos.ranks.* §cможет изменять только Владелец или Консоль."
                    .into(),
            );
        }

        match action.as_str() {
            "check" => {
                let target_prof = get_or_default_profile(&st.store, target_name, now);
                let allowed = has_permission(&st.config, &target_prof, node);
                Ok((
                    format!(
                        "§6§l✦ KRONOS RANKS §8» §7Право §f{} §7у игрока §f{}§7: {}",
                        node,
                        target_name,
                        if allowed { "§a✔ ЕСТЬ" } else { "§c✖ НЕТ" }
                    ),
                    false,
                ))
            }
            "add" => {
                let added = {
                    let target_prof = ensure_profile(&mut st.store, target_name, now);
                    if !target_prof
                        .extra_permissions
                        .iter()
                        .any(|p| p.eq_ignore_ascii_case(node))
                    {
                        target_prof.extra_permissions.push(node.to_string());
                        target_prof.updated_at_ms = now;
                        true
                    } else {
                        false
                    }
                };
                if added {
                    push_audit(
                        &mut st.store,
                        &actor_name,
                        target_name,
                        "PERM_ADD",
                        node,
                        "навсегда".into(),
                        now,
                    );
                    st.dirty = true;
                }
                Ok((
                    format!(
                        "§6§l✦ KRONOS RANKS §8» §aИгроку §f{} §aвыдано право §e{}§a.",
                        target_name, node
                    ),
                    true,
                ))
            }
            "remove" => {
                let removed = {
                    let target_prof = ensure_profile(&mut st.store, target_name, now);
                    let before = target_prof.extra_permissions.len();
                    target_prof
                        .extra_permissions
                        .retain(|p| !p.eq_ignore_ascii_case(node));
                    if target_prof.extra_permissions.len() != before {
                        target_prof.updated_at_ms = now;
                        true
                    } else {
                        false
                    }
                };
                if removed {
                    push_audit(
                        &mut st.store,
                        &actor_name,
                        target_name,
                        "PERM_REMOVE",
                        node,
                        "-".into(),
                        now,
                    );
                    st.dirty = true;
                }
                Ok((
                    format!(
                        "§6§l✦ KRONOS RANKS §8» §eУ игрока §f{} §eотозвано право §f{}§e.",
                        target_name, node
                    ),
                    true,
                ))
            }
            _ => Err("§cИспользование: §f/rank perm <игрок> <add|remove|check> <право>".into()),
        }
    });

    match res {
        Ok((msg, mutated)) => {
            if mutated {
                save_current_store();
                refresh_online_player_by_name(server, target_name);
                if action == "remove" {
                    if let Some(target_player) = find_online_player_ci(server, target_name) {
                        target_player.unset_permission(node);
                    }
                }
            }
            if let Some(p) = sender.as_player() {
                play_success_sound(&p);
            }
            sender.send_system_message(legacy(&msg));
        }
        Err(err) => {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(&err));
        }
    }
}

fn strip_legacy_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            let _ = chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

// =============================================================================
// Command Registration
// =============================================================================

pub fn register_all(context: &Context) {
    let donate = Command::new(
        &["donate".into(), "ranks".into()],
        "Интерактивная витрина 10 донат-привилегий KRONOS",
    )
    .execute(DonateCmd);
    context.register_command(donate, PERM_PLAYER);

    let staff = Command::new(
        &["staff".into()],
        "Отделы стаффа, онлайн-сотрудники, заявки HR и аудит (/staff)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(StaffCmd),
    )
    .execute(StaffCmd);
    context.register_command(staff, PERM_PLAYER);

    let staffchat = Command::new(
        &["sc".into(), "staffchat".into()],
        "Закрытый чат стаффа KRONOS (/sc <сообщение>)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(StaffChatCmd),
    )
    .execute(StaffChatCmd);
    context.register_command(staffchat, PERM_PLAYER);

    let rank = Command::new(
        &["rank".into()],
        "Управление рангами, профилем, суффиксом и правами (/rank)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(RankCmd { direct_set: false }),
    )
    .execute(RankCmd { direct_set: false });
    context.register_command(rank, PERM_PLAYER);

    let setrank = Command::new(
        &["setrank".into()],
        "Быстрая выдача ранга игроку (/setrank <игрок> <ранг> [срок])",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(RankCmd { direct_set: true }),
    )
    .execute(RankCmd { direct_set: true });
    context.register_command(setrank, PERM_PLAYER);
}
