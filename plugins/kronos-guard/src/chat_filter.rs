use crate::host;
use crate::models::{
    format_duration_ru, parse_duration_seconds, GuardConfig, GuardStore, PunishmentKind,
    RuntimeSession,
};
use crate::punishments::{active_mute, issue_punishment, normalize_key, notify_staff};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatDecision {
    Allow,
    Block { reason: String, add_strike: bool },
}

/// Evaluates an incoming chat message against active mutes, anti-spam, anti-caps, anti-advertising, and swear list.
pub fn evaluate_chat(
    store: &mut GuardStore,
    config: &GuardConfig,
    session: &mut RuntimeSession,
    dirty: &mut bool,
    player_id: &str,
    player_name: &str,
    message: &str,
    now_ms: u64,
) -> ChatDecision {
    // 1. Active mute check (always enforced)
    if let Some(mute) = active_mute(store, player_name, now_ms) {
        let rem = match mute.remaining_ms(now_ms) {
            None => "навсегда".to_string(),
            Some(ms) => format_duration_ru(ms / 1000),
        };
        host::send_chat(
            player_id,
            &format!(
                "§c✗ Вы заглушены (§e{}§c). Причина: §f{}",
                rem, mute.reason
            ),
        );
        host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
        return ChatDecision::Block {
            reason: "active_mute".into(),
            add_strike: false,
        };
    }

    if !config.chat_filter.enabled {
        return ChatDecision::Allow;
    }

    let rank = host::get_player_rank(player_id);
    if config.perms_for(&rank).bypass_chat_filter {
        return ChatDecision::Allow;
    }

    let key = normalize_key(player_name);
    let decision = inspect_message_rules(&config.chat_filter, session, &key, message, now_ms);

    if let ChatDecision::Block {
        ref reason,
        add_strike,
    } = decision
    {
        host::send_chat(player_id, &format!("§c[GUARD] {}", reason));
        host::play_sound(player_id, "minecraft:block.note_block.bass", 0.9, 0.6);

        if add_strike {
            let strikes = {
                let tr = session.chat_trackers.entry(key.clone()).or_default();
                tr.strikes += 1;
                tr.strikes
            };

            notify_staff(
                config,
                session,
                store,
                &format!(
                    "§8[§cChatGuard§8] §f{} §7заблокирован ({}§7) §8[§e{}/{}§8]: §7\"{}\"",
                    player_name,
                    reason,
                    strikes,
                    config.chat_filter.auto_mute_strikes,
                    message
                ),
            );

            if config.chat_filter.auto_mute_strikes > 0
                && strikes >= config.chat_filter.auto_mute_strikes
            {
                if let Some(tr) = session.chat_trackers.get_mut(&key) {
                    tr.strikes = 0;
                }
                let mute_secs =
                    parse_duration_seconds(&config.chat_filter.auto_mute_duration).unwrap_or(Some(900));
                let _ = issue_punishment(
                    store,
                    config,
                    session,
                    dirty,
                    "CONSOLE",
                    "KRONOS ChatGuard",
                    "owner",
                    PunishmentKind::Mute,
                    player_name,
                    mute_secs,
                    "Авто-мут: спам / реклама / нарушения в чате",
                    now_ms,
                );
            }
        }
    }

    decision
}

fn inspect_message_rules(
    cfg: &crate::models::ChatFilterConfig,
    session: &mut RuntimeSession,
    player_key: &str,
    message: &str,
    now_ms: u64,
) -> ChatDecision {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return ChatDecision::Allow;
    }

    // Anti-advertising (IPv4, URLs, obfuscated domains, Telegram/Discord invites)
    if cfg.block_advertising && contains_advertisement(trimmed, &cfg.whitelisted_domains) {
        return ChatDecision::Block {
            reason: "Реклама сторонних серверов, IP и ссылок запрещена!".into(),
            add_strike: true,
        };
    }

    // Swear / blocked words check
    let lowered = trimmed.to_lowercase();
    for bad in &cfg.blocked_words {
        if !bad.is_empty() && lowered.contains(&bad.to_lowercase()) {
            return ChatDecision::Block {
                reason: "Нецензурная лексика в чате запрещена.".into(),
                add_strike: true,
            };
        }
    }

    // Anti-Caps check
    let letters: Vec<char> = trimmed.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() >= cfg.min_caps_length {
        let upper = letters.iter().filter(|c| c.is_uppercase()).count();
        let ratio = upper as f32 / letters.len() as f32;
        if ratio >= cfg.max_caps_ratio {
            return ChatDecision::Block {
                reason: "Выключите CapsLock — слишком много заглавных букв.".into(),
                add_strike: false,
            };
        }
    }

    // Anti-Spam: cooldown, duplicate, burst flood
    let tracker = session
        .chat_trackers
        .entry(player_key.to_string())
        .or_default();

    if cfg.cooldown_ms > 0 && now_ms.saturating_sub(tracker.last_message_ms) < cfg.cooldown_ms {
        return ChatDecision::Block {
            reason: "Не отправляйте сообщения так часто!".into(),
            add_strike: false,
        };
    }

    if cfg.block_duplicates && !tracker.last_message_text.is_empty() && tracker.last_message_text == lowered {
        return ChatDecision::Block {
            reason: "Не дублируйте одинаковые сообщения в чат.".into(),
            add_strike: true,
        };
    }

    tracker
        .recent_timestamps
        .retain(|ts| now_ms.saturating_sub(*ts) <= cfg.burst_window_ms);
    tracker.recent_timestamps.push(now_ms);
    if cfg.burst_max_messages > 0 && tracker.recent_timestamps.len() > cfg.burst_max_messages {
        return ChatDecision::Block {
            reason: "Обнаружен флуд! Сбавьте темп сообщений.".into(),
            add_strike: true,
        };
    }

    tracker.last_message_ms = now_ms;
    tracker.last_message_text = lowered;

    ChatDecision::Allow
}

/// Detects IPv4 addresses, URLs, Discord/Telegram invites, and obfuscated domain names
/// while allowing domains in `whitelisted_domains`.
pub fn contains_advertisement(message: &str, whitelisted_domains: &[String]) -> bool {
    let lower = message.to_lowercase();

    // Strip whitelisted domains before scanning
    let mut sanitized = lower.clone();
    for allowed in whitelisted_domains {
        let a = allowed.to_lowercase();
        if !a.is_empty() {
            sanitized = sanitized.replace(&a, " ");
        }
    }

    // 1. Direct invite / protocol prefixes
    let instant_markers = [
        "http://",
        "https://",
        "discord.gg/",
        "discord.com/invite",
        "t.me/",
        "vk.com/",
        "youtu.be/",
        "twitch.tv/",
    ];
    if instant_markers.iter().any(|m| sanitized.contains(m)) {
        return true;
    }

    // 2. De-obfuscate common bypasses: "(dot)", "[.]", spaces around dots/commas
    let compact: String = sanitized
        .replace("(dot)", ".")
        .replace("[dot]", ".")
        .replace("[.]", ".")
        .replace("(.)", ".")
        .replace(',', ".")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    // 3. IPv4 detection (four 1..3-digit numeric segments separated by '.')
    if has_ipv4(&compact) {
        return true;
    }

    // 4. Domain TLD detection (word.tld where tld is a known server/web zone)
    let tlds = [
        ".ru", ".com", ".net", ".org", ".su", ".pro", ".fun", ".me", ".xyz", ".gg", ".mc", ".ua",
        ".kz", ".by", ".top", ".pw", ".io", ".site", ".store",
    ];
    for tld in &tlds {
        if let Some(idx) = compact.find(tld) {
            if idx >= 2 {
                let prev = compact[..idx].chars().last().unwrap_or(' ');
                let after = compact[idx + tld.len()..].chars().next();
                let valid_after = match after {
                    None => true,
                    Some(c) => !c.is_alphanumeric(),
                };
                if prev.is_alphanumeric() && valid_after {
                    return true;
                }
            }
        }
    }

    false
}

fn has_ipv4(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut segments = 0u8;
            let mut j = i;
            loop {
                let start = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                let seg_len = j - start;
                if seg_len == 0 || seg_len > 3 {
                    break;
                }
                if let Ok(val) = text[start..j].parse::<u16>() {
                    if val > 255 {
                        break;
                    }
                } else {
                    break;
                }
                segments += 1;
                if segments == 4 {
                    return true;
                }
                if j < bytes.len() && bytes[j] == b'.' {
                    j += 1;
                } else {
                    break;
                }
            }
        }
        i += 1;
    }
    false
}
