use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffRankPerms {
    pub weight: u32,
    pub prefix: String,
    pub can_warn: bool,
    pub can_mute: bool,
    /// 0 = unlimited
    pub max_mute_seconds: u64,
    pub can_kick: bool,
    pub can_tempban: bool,
    /// 0 = unlimited
    pub max_ban_seconds: u64,
    pub can_permban: bool,
    pub can_ipban: bool,
    pub can_unpunish: bool,
    pub can_vanish: bool,
    pub can_freeze: bool,
    pub can_inspect: bool,
    pub can_rollback: bool,
    pub can_staff_chat: bool,
    pub bypass_chat_filter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarnEscalationRule {
    pub warn_count: u32,
    /// "mute" | "tempban" | "ban"
    pub action: String,
    pub duration: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFilterConfig {
    pub enabled: bool,
    pub cooldown_ms: u64,
    pub burst_window_ms: u64,
    pub burst_max_messages: usize,
    pub block_duplicates: bool,
    pub max_caps_ratio: f32,
    pub min_caps_length: usize,
    pub block_advertising: bool,
    pub whitelisted_domains: Vec<String>,
    pub blocked_words: Vec<String>,
    pub auto_mute_strikes: u32,
    pub auto_mute_duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockLogConfig {
    pub enabled: bool,
    pub max_entries: usize,
    pub retention_days: u64,
    pub default_lookup_radius: i32,
    pub max_lookup_radius: i32,
    pub max_rollback_radius: i32,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VanishConfig {
    pub fake_leave_join_messages: bool,
    pub silent_containers: bool,
    pub disable_pickup_default: bool,
    pub show_action_bar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    pub server_name: String,
    pub appeal_url: String,
    pub autosave_interval_seconds: u64,
    pub inspector_item: String,
    pub freeze_logout_ban_duration: String,
    pub report_cooldown_seconds: u64,
    pub staff_ranks: HashMap<String, StaffRankPerms>,
    pub warn_escalation: Vec<WarnEscalationRule>,
    pub chat_filter: ChatFilterConfig,
    pub block_log: BlockLogConfig,
    pub vanish: VanishConfig,
}

impl GuardConfig {
    pub fn perms_for(&self, rank: &str) -> StaffRankPerms {
        let key = rank.to_lowercase();
        self.staff_ranks
            .get(&key)
            .or_else(|| self.staff_ranks.get("player"))
            .cloned()
            .unwrap_or(StaffRankPerms {
                weight: 0,
                prefix: "§7[Игрок]".into(),
                can_warn: false,
                can_mute: false,
                max_mute_seconds: 0,
                can_kick: false,
                can_tempban: false,
                max_ban_seconds: 0,
                can_permban: false,
                can_ipban: false,
                can_unpunish: false,
                can_vanish: false,
                can_freeze: false,
                can_inspect: false,
                can_rollback: false,
                can_staff_chat: false,
                bypass_chat_filter: false,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PunishmentKind {
    Ban,
    TempBan,
    IpBan,
    Mute,
    Warn,
    Kick,
}

impl PunishmentKind {
    pub fn label_ru(self) -> &'static str {
        match self {
            PunishmentKind::Ban => "§4БАН",
            PunishmentKind::TempBan => "§cТЕМПБАН",
            PunishmentKind::IpBan => "§4§lIP-БАН",
            PunishmentKind::Mute => "§6МУТ",
            PunishmentKind::Warn => "§eВАРН",
            PunishmentKind::Kick => "§7КИК",
        }
    }

    pub fn is_ban(self) -> bool {
        matches!(self, PunishmentKind::Ban | PunishmentKind::TempBan | PunishmentKind::IpBan)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PunishmentRecord {
    pub id: u64,
    pub kind: PunishmentKind,
    pub target_key: String,
    pub target_name: String,
    pub target_ip: Option<String>,
    pub staff_id: String,
    pub staff_name: String,
    pub reason: String,
    pub created_at_ms: u64,
    /// None = permanent
    pub expires_at_ms: Option<u64>,
    pub revoked_by: Option<String>,
    pub revoked_at_ms: Option<u64>,
}

impl PunishmentRecord {
    pub fn is_active(&self, now_ms: u64) -> bool {
        if self.revoked_at_ms.is_some() {
            return false;
        }
        match self.expires_at_ms {
            None => true,
            Some(exp) => now_ms < exp,
        }
    }

    pub fn remaining_ms(&self, now_ms: u64) -> Option<u64> {
        self.expires_at_ms.map(|exp| exp.saturating_sub(now_ms))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockAction {
    Break = 0,
    Place = 1,
    ContainerOpen = 2,
    ContainerTake = 3,
    ContainerPut = 4,
    Explosion = 5,
    LavaPlace = 6,
    WaterPlace = 7,
}

impl BlockAction {
    pub fn symbol_ru(self) -> (&'static str, &'static str) {
        match self {
            BlockAction::Break => ("§c[-]", "сломал"),
            BlockAction::Place => ("§a[+]", "поставил"),
            BlockAction::ContainerOpen => ("§e[?]", "открыл"),
            BlockAction::ContainerTake => ("§c[↑]", "забрал из"),
            BlockAction::ContainerPut => ("§a[↓]", "положил в"),
            BlockAction::Explosion => ("§4[✹]", "взорвал"),
            BlockAction::LavaPlace => ("§6[~]", "разлил лаву"),
            BlockAction::WaterPlace => ("§b[~]", "разлил воду"),
        }
    }
}

/// Compact block & container audit record designed for minimal RAM/JSON footprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockLogEntry {
    /// Unix timestamp in seconds
    pub ts: u64,
    /// World name (e.g. "world")
    pub w: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Player name or "#tnt"/"#creeper"
    pub actor: String,
    pub action: BlockAction,
    /// Block ID or container ID
    pub block: String,
    /// Optional item detail for container take/put (e.g. "diamond x12")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// Previous block for rollback support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_block: Option<String>,
    pub rolled_back: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackOp {
    pub world: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub before_block: String,
    pub after_block: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackBatch {
    pub id: u64,
    pub staff_name: String,
    pub created_at_ms: u64,
    pub ops: Vec<RollbackOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEntry {
    pub id: u64,
    pub reporter_name: String,
    pub target_name: String,
    pub reason: String,
    pub world: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub created_at_ms: u64,
    pub resolved: bool,
    pub resolved_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuardStore {
    pub next_punishment_id: u64,
    pub next_report_id: u64,
    pub next_rollback_id: u64,
    pub punishments: Vec<PunishmentRecord>,
    pub reports: Vec<ReportEntry>,
    pub known_ips: HashMap<String, String>,
    pub known_names: HashMap<String, String>,
    pub known_ranks: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct ChatTracker {
    pub last_message_ms: u64,
    pub last_message_text: String,
    pub recent_timestamps: Vec<u64>,
    pub strikes: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeSession {
    pub vanished: HashSet<String>,
    pub inspectors: HashSet<String>,
    pub frozen: HashMap<String, String>, // target_key -> staff_name
    pub online_ids: HashMap<String, String>, // lower_name -> player_id
    pub chat_trackers: HashMap<String, ChatTracker>,
    pub report_cooldowns: HashMap<String, u64>,
    pub rollback_history: HashMap<String, Vec<RollbackBatch>>,
    pub last_inspected_pos: HashMap<String, (String, i32, i32, i32)>,
}

#[derive(Debug, Clone)]
pub struct PlayerPos {
    pub world: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Parses human durations such as "30s", "15m", "2h", "7d", "2w", "1mo", "perm".
/// Returns Ok(None) for permanent, Ok(Some(seconds)) for timed, Err(()) for invalid input.
pub fn parse_duration_seconds(input: &str) -> Result<Option<u64>, ()> {
    let raw = input.trim().to_lowercase();
    if raw.is_empty() {
        return Err(());
    }
    if matches!(raw.as_str(), "perm" | "permanent" | "forever" | "навсегда") {
        return Ok(None);
    }

    if let Some(num) = raw.strip_suffix("mo") {
        let v: u64 = num.parse().map_err(|_| ())?;
        return if v > 0 { Ok(Some(v.saturating_mul(2_592_000))) } else { Err(()) };
    }

    let (num_part, unit) = raw.split_at(raw.len() - 1);
    let value: u64 = num_part.parse().map_err(|_| ())?;
    if value == 0 {
        return Err(());
    }

    let mult: u64 = match unit {
        "s" | "с" => 1,
        "m" | "м" => 60,
        "h" | "ч" => 3600,
        "d" | "д" => 86_400,
        "w" | "н" => 604_800,
        _ => return Err(()),
    };

    Ok(Some(value.saturating_mul(mult)))
}

/// Formats seconds into a compact Russian string like "2д 4ч 15м" or "45с".
pub fn format_duration_ru(total_seconds: u64) -> String {
    if total_seconds == 0 {
        return "0с".into();
    }
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3600;
    let mins = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{}д", days));
    }
    if hours > 0 {
        parts.push(format!("{}ч", hours));
    }
    if mins > 0 {
        parts.push(format!("{}м", mins));
    }
    if secs > 0 && days == 0 {
        parts.push(format!("{}с", secs));
    }
    if parts.is_empty() {
        "1с".into()
    } else {
        parts.join(" ")
    }
}

pub fn format_ago_ru(now_sec: u64, event_sec: u64) -> String {
    let diff = now_sec.saturating_sub(event_sec);
    if diff < 60 {
        format!("{}с назад", diff)
    } else if diff < 3600 {
        format!("{}м назад", diff / 60)
    } else if diff < 86_400 {
        format!("{}ч назад", diff / 3600)
    } else {
        format!("{}д назад", diff / 86_400)
    }
}
