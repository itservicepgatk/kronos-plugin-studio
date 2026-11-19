use crate::hierarchy;
use crate::host;
use crate::models::{PlayerRankProfile, RanksConfig};

/// Applies Tablist prefix/suffix/sort-order, Header/Footer, and syncs primary rank to Pumpkin host.
pub fn refresh_player_display(
    config: &RanksConfig,
    player_id: &str,
    profile: &PlayerRankProfile,
    online_count: usize,
) {
    let primary = hierarchy::primary_rank(config, profile);
    let suffix = profile
        .custom_suffix
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&primary.suffix);

    host::sync_host_rank(player_id, &primary.id);
    host::set_player_tablist(player_id, &primary.prefix, suffix, primary.weight);

    let header = config
        .tab_header
        .replace("{online}", &online_count.to_string())
        .replace("{prefix}", primary.prefix.trim())
        .replace("{department}", primary.department.label_ru());
    let footer = config
        .tab_footer
        .replace("{online}", &online_count.to_string())
        .replace("{prefix}", primary.prefix.trim())
        .replace("{department}", primary.department.label_ru());

    host::set_tab_header_footer(player_id, &header, &footer);
}

/// Formats a chat message with rank prefix, player name color, optional custom suffix, and color-code support for VIP+.
pub fn format_chat_line(
    config: &RanksConfig,
    profile: &PlayerRankProfile,
    raw_message: &str,
) -> String {
    let primary = hierarchy::primary_rank(config, profile);
    let perks = hierarchy::effective_perks(config, profile);

    let suffix = profile
        .custom_suffix
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| format!(" {}", s))
        .unwrap_or_else(|| primary.suffix.clone());

    let rendered_msg = if perks.colored_chat {
        translate_ampersand_colors(raw_message, perks.rgb_chat)
    } else {
        raw_message.to_string()
    };

    format!(
        "{}{}{}{}§8 » {}{}",
        primary.prefix, primary.name_color, profile.player_name, suffix, primary.chat_color, rendered_msg
    )
}

/// Converts `&a`, `&6`, etc. into Minecraft `§` codes for ranks with `colored_chat`.
pub fn translate_ampersand_colors(input: &str, allow_bold_and_hex: bool) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' && i + 1 < chars.len() {
            let next = chars[i + 1].to_ascii_lowercase();
            let basic = matches!(
                next,
                '0'..='9' | 'a'..='f' | 'r'
            );
            let advanced = allow_bold_and_hex && matches!(next, 'l' | 'o' | 'n' | 'm' | '#');
            if basic || advanced {
                out.push('§');
                out.push(next);
                i += 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Announces high-tier donate or staff joins with special audio/title flair.
pub fn announce_join_flair(config: &RanksConfig, player_id: &str, profile: &PlayerRankProfile) {
    let primary = hierarchy::primary_rank(config, profile);
    if primary.id == "kronos" || primary.id == "owner" {
        host::broadcast_chat(&format!(
            "§6§l✦ §r{}{} §eвошёл на сервер!",
            primary.prefix, profile.player_name
        ));
        host::play_sound(player_id, "minecraft:ui.toast.challenge_complete", 0.9, 1.1);
    } else if hierarchy::has_permission(config, profile, "kronos.join.title_burst") {
        host::broadcast_chat(&format!(
            "§e★ {}{} §7присоединился к игре.",
            primary.prefix, profile.player_name
        ));
        host::play_sound(player_id, "minecraft:entity.player.levelup", 0.8, 1.2);
    } else if hierarchy::has_permission(config, profile, "kronos.join.announce") {
        host::broadcast_chat(&format!(
            "§8[§a+§8] {}{}",
            primary.prefix, profile.player_name
        ));
    }
}
