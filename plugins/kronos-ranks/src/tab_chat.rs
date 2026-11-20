use pumpkin_plugin_api::{
    text::TextComponent,
    wit::pumpkin::plugin::sounds::SoundCategory,
    Player, Server,
};

use crate::commands::{PERM_ADMIN, PERM_STAFF};
use crate::hierarchy::{
    collect_rank_permissions, effective_perks, has_permission, is_staff_member, normalize_key,
    primary_rank,
};
use crate::models::{PlayerRankProfile, RanksConfig};
use crate::storage::{get_or_default_profile, now_ms, with_state};

pub const SOUND_SUCCESS: &str = "minecraft:block.note_block.pling";
pub const SOUND_CHIME: &str = "minecraft:block.note_block.chime";
pub const SOUND_CLICK: &str = "minecraft:ui.button.click";
pub const SOUND_ERROR: &str = "minecraft:entity.villager.no";
pub const SOUND_RANK_UP: &str = "minecraft:entity.player.levelup";

const TAB_EMBLEM_FRAMES: [&str; 4] = ["§6§l✦", "§e§l✧", "§6§l★", "§e§l✦"];

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

pub fn play_success_sound(player: &Player) {
    player.play_custom_sound(SOUND_SUCCESS, SoundCategory::Players, 0.9, 1.45);
}

pub fn play_chime_sound(player: &Player) {
    player.play_custom_sound(SOUND_CHIME, SoundCategory::Players, 1.0, 1.25);
}

pub fn play_click_sound(player: &Player) {
    player.play_custom_sound(SOUND_CLICK, SoundCategory::Players, 0.7, 1.2);
}

pub fn play_error_sound(player: &Player) {
    player.play_custom_sound(SOUND_ERROR, SoundCategory::Players, 0.9, 0.9);
}

pub fn play_rank_up_sound(player: &Player) {
    player.play_custom_sound(SOUND_RANK_UP, SoundCategory::Players, 1.0, 1.15);
}

pub fn resolved_suffix(config: &RanksConfig, profile: &PlayerRankProfile) -> String {
    let primary = primary_rank(config, profile);
    let perks = effective_perks(config, profile);
    if perks.custom_suffix {
        if let Some(ref custom) = profile.custom_suffix {
            if !custom.trim().is_empty() {
                return format!(" {}", custom.trim());
            }
        }
    }
    if primary.suffix.trim().is_empty() {
        String::new()
    } else {
        format!(" {}", primary.suffix.trim())
    }
}

/// Builds the formatted Tab & display name string: `{prefix}{name_color}{name}{suffix}`.
pub fn build_tab_name(config: &RanksConfig, profile: &PlayerRankProfile) -> String {
    let primary = primary_rank(config, profile);
    let suffix = resolved_suffix(config, profile);
    format!(
        "{}{}{}{}",
        primary.prefix, primary.name_color, profile.player_name, suffix
    )
}

/// Computes the Tablist sort priority integer for `player.set_tab_list_order(order)`.
/// Higher rank weight yields a higher priority integer so Owner/Curator appear above VIP/Player.
pub fn tab_sort_order(config: &RanksConfig, profile: &PlayerRankProfile) -> i32 {
    primary_rank(config, profile).weight as i32
}

/// Builds the animated Tablist header & footer strings with live online count, ping, rank, and store link.
pub fn build_tab_header_footer(
    config: &RanksConfig,
    profile: &PlayerRankProfile,
    online_count: u32,
    max_players: u32,
    ping_ms: u32,
    anim_frame: usize,
) -> (String, String) {
    let primary = primary_rank(config, profile);
    let emblem = TAB_EMBLEM_FRAMES[anim_frame % TAB_EMBLEM_FRAMES.len()];

    let ping_color = if ping_ms <= 60 {
        "§a"
    } else if ping_ms <= 140 {
        "§e"
    } else {
        "§c"
    };
    let ping_label = format!("{}{}ms", ping_color, ping_ms);

    let replace_vars = |tpl: &str| -> String {
        tpl.replace("{online}", &online_count.to_string())
            .replace("{max_players}", &max_players.to_string())
            .replace("{ping}", &ping_label)
            .replace("{prefix}", primary.prefix.trim())
            .replace("{rank}", &primary.display_name)
            .replace("{department}", primary.department.label_ru())
            .replace("{store_url}", &config.store_url)
            .replace("{server}", &config.server_name)
    };

    let raw_header = replace_vars(&config.tab_header);
    let header = if raw_header.contains("§6§l✦") {
        raw_header.replacen("§6§l✦", emblem, 1)
    } else {
        format!("{} {}", emblem, raw_header)
    };

    let mut footer = replace_vars(&config.tab_footer);
    if !config.tab_footer.contains("{ping}") {
        footer.push_str(&format!(
            "\n§7Пинг: {} §8| §7Ранг: {} §8| §7Онлайн: §e{}/{}",
            ping_label,
            primary.prefix.trim(),
            online_count,
            max_players
        ));
    }

    (header, footer)
}

/// Applies Tablist name, Tablist priority order, display name, and animated header/footer to one player.
/// Must NEVER be called while holding the `STATE` Mutex!
pub fn refresh_player_display(
    player: &Player,
    config: &RanksConfig,
    profile: &PlayerRankProfile,
    online_count: u32,
    max_players: u32,
    anim_frame: usize,
) {
    let tab_name = build_tab_name(config, profile);
    let order = tab_sort_order(config, profile);
    let ping = player.get_ping();

    player.set_tab_list_name(Some(legacy(&tab_name)));
    player.set_tab_list_order(order);
    player.set_display_name(legacy(&tab_name));

    let (header, footer) =
        build_tab_header_footer(config, profile, online_count, max_players, ping, anim_frame);
    player.set_tab_list_header_footer(legacy(&header), legacy(&footer));
}

/// Synchronizes KRONOS rank permissions onto Pumpkin's native `Player` permission table.
/// Must NEVER be called while holding the `STATE` Mutex!
pub fn sync_player_native_permissions(
    player: &Player,
    config: &RanksConfig,
    profile: &PlayerRankProfile,
) {
    let staff = is_staff_member(config, profile);
    let primary = primary_rank(config, profile);
    let is_admin = primary.weight >= 700 || has_permission(config, profile, "*");

    if staff {
        player.set_permission(PERM_STAFF, true);
    } else if player.has_permission_set(PERM_STAFF) == Some(true) {
        player.unset_permission(PERM_STAFF);
    }

    if is_admin {
        player.set_permission(PERM_ADMIN, true);
    } else if player.has_permission_set(PERM_ADMIN) == Some(true) {
        player.unset_permission(PERM_ADMIN);
    }

    // Sync explicit concrete permission nodes from donate, staff, and extra permissions
    for node in collect_rank_permissions(config, &profile.donate_rank) {
        if !node.contains('*') {
            player.set_permission(&node, true);
        }
    }
    if let Some(ref staff_id) = profile.staff_rank {
        for node in collect_rank_permissions(config, staff_id) {
            if !node.contains('*') {
                player.set_permission(&node, true);
            }
        }
    }
    for node in &profile.extra_permissions {
        if !node.contains('*') {
            player.set_permission(node, true);
        }
    }
}

/// Refreshes Tablist display (name, sort order, header/footer) for all currently online players.
/// Snapshots state first and releases the Mutex BEFORE making any WIT host calls.
pub fn refresh_all_online_tabs(server: &Server) {
    let players = server.get_all_players();
    if players.is_empty() {
        return;
    }

    let online_count = players.len() as u32;
    let max_players = server.get_max_players();
    let now = now_ms();

    let (config, profiles, anim_frame) =
        with_state(|st| (st.config.clone(), st.store.profiles.clone(), st.anim_frame));

    for player in &players {
        let p_name = player.get_name();
        let key = normalize_key(&p_name);
        let profile = profiles
            .get(&key)
            .cloned()
            .unwrap_or_else(|| PlayerRankProfile::new(&p_name, now));
        refresh_player_display(player, &config, &profile, online_count, max_players, anim_frame);
    }
}

/// Refreshes only the animated header/footer (plus ping & online count) for all online players.
pub fn refresh_all_headers_footers(server: &Server) {
    let players = server.get_all_players();
    if players.is_empty() {
        return;
    }

    let online_count = players.len() as u32;
    let max_players = server.get_max_players();
    let now = now_ms();

    let (config, profiles, anim_frame) =
        with_state(|st| (st.config.clone(), st.store.profiles.clone(), st.anim_frame));

    for player in &players {
        let p_name = player.get_name();
        let key = normalize_key(&p_name);
        let profile = profiles
            .get(&key)
            .cloned()
            .unwrap_or_else(|| PlayerRankProfile::new(&p_name, now));
        let ping = player.get_ping();
        let (header, footer) =
            build_tab_header_footer(&config, &profile, online_count, max_players, ping, anim_frame);
        player.set_tab_list_header_footer(legacy(&header), legacy(&footer));
    }
}

/// Refreshes a single online player by name (if online), including Tablist and native permissions.
pub fn refresh_online_player_by_name(server: &Server, target_name: &str) {
    let Some(target) = find_online_player_ci(server, target_name) else {
        return;
    };
    let actual_name = target.get_name();
    let now = now_ms();
    let (config, profile, anim_frame) = with_state(|st| {
        let prof = get_or_default_profile(&st.store, &actual_name, now);
        (st.config.clone(), prof, st.anim_frame)
    });

    let online_count = server.get_player_count().max(1);
    let max_players = server.get_max_players();
    sync_player_native_permissions(&target, &config, &profile);
    refresh_player_display(&target, &config, &profile, online_count, max_players, anim_frame);
}

/// Case-insensitive lookup for an online player on the server.
pub fn find_online_player_ci(server: &Server, name: &str) -> Option<Player> {
    if let Some(p) = server.get_player_by_name(name) {
        return Some(p);
    }
    let needle = normalize_key(name);
    server
        .get_all_players()
        .into_iter()
        .find(|p| normalize_key(&p.get_name()) == needle)
}

/// Translates `&` color codes safely according to the player's `colored_chat` / `rgb_chat` perks.
pub fn sanitize_and_color_message(
    raw: &str,
    allow_colors: bool,
    allow_styles: bool,
    allow_obfuscated: bool,
) -> String {
    // Strip raw '§' from player input so regular players cannot spoof colors
    let cleaned = raw.replace('§', "");
    if !allow_colors {
        return cleaned;
    }

    let chars: Vec<char> = cleaned.chars().collect();
    let mut out = String::with_capacity(cleaned.len());
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '&' && i + 1 < chars.len() {
            let code = chars[i + 1].to_ascii_lowercase();
            let is_color = matches!(
                code,
                '0'..='9' | 'a'..='f' | 'r'
            );
            let is_style = matches!(code, 'l' | 'm' | 'n' | 'o');
            let is_magic = code == 'k';

            if is_color || (allow_styles && is_style) || (allow_obfuscated && is_magic) {
                out.push('§');
                out.push(code);
                i += 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Formats public chat line: `{Prefix}{NameColor}{Name}{Suffix} §8» {ChatColor}{Message}`
pub fn format_chat_line(
    config: &RanksConfig,
    profile: &PlayerRankProfile,
    raw_message: &str,
) -> String {
    let primary = primary_rank(config, profile);
    let perks = effective_perks(config, profile);
    let suffix = resolved_suffix(config, profile);
    let is_staff = is_staff_member(config, profile);

    let msg = sanitize_and_color_message(
        raw_message,
        perks.colored_chat || is_staff,
        perks.rgb_chat || is_staff,
        primary.weight >= 700,
    );

    format!(
        "{}{}{}{} §8» {}{}",
        primary.prefix, primary.name_color, profile.player_name, suffix, primary.chat_color, msg
    )
}

/// Formats private staff chat line (`/sc`): `§8[§b§lSC§8] {Prefix}{NameColor}{Name} §8» §f{Message}`
pub fn format_staff_chat_line(
    config: &RanksConfig,
    profile: &PlayerRankProfile,
    raw_message: &str,
) -> String {
    let primary = primary_rank(config, profile);
    let msg = sanitize_and_color_message(raw_message, true, true, false);
    format!(
        "§8[§b§lSC§8] {}{}{} §8» §f{}",
        primary.prefix, primary.name_color, profile.player_name, msg
    )
}

/// Broadcasts a formatted system message to all online players.
pub fn broadcast_component(server: &Server, component: TextComponent) {
    let text_json = component.to_json();
    for player in server.get_all_players() {
        let msg = TextComponent::from_json(&text_json).unwrap_or_else(|_| legacy(&component.get_text()));
        player.send_system_message(msg, false);
    }
}

/// Sends a formatted join announcement and welcome title for Elite+ donate ranks & Staff.
pub fn announce_join_flair(
    server: &Server,
    player: &Player,
    config: &RanksConfig,
    profile: &PlayerRankProfile,
) {
    let primary = primary_rank(config, profile);
    if primary.weight >= 25 {
        let line = if primary.weight >= 100 {
            format!(
                "§6§l✦ §7Сотрудник {}{}{} §7({}) §aвошёл на сервер!",
                primary.prefix,
                primary.name_color,
                profile.player_name,
                primary.department.label_ru()
            )
        } else {
            format!(
                "§6§l✦ {}{}{} §eприсоединился к серверу!",
                primary.prefix, primary.name_color, profile.player_name
            )
        };
        for recipient in server.get_all_players() {
            recipient.send_system_message(legacy(&line), false);
        }
    }

    if primary.weight >= 45 {
        player.show_title(legacy(&format!("{}§r", primary.prefix.trim())));
        player.show_subtitle(legacy(&format!(
            "§7Добро пожаловать на §6{}§7!",
            config.server_name
        )));
        play_chime_sound(player);
    }
}
