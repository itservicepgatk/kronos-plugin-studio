use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

use pumpkin_plugin_api::{
    Context, Player, Plugin, PluginMetadata, Server,
    command::{ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType},
    commands::CommandHandler,
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    text::TextComponent,
    wit::pumpkin::plugin::{
        command::Arg,
        sounds::SoundCategory,
    },
};
use serde::{Deserialize, Serialize};

const PERM_PLAYER: &str = "kronos-essentials:player";
const PERM_ADMIN: &str = "kronos-essentials:admin";

fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LocationPoint {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct EssentialsStore {
    pub homes: HashMap<String, HashMap<String, LocationPoint>>,
    pub warps: HashMap<String, LocationPoint>,
    pub spawn: Option<LocationPoint>,
    pub last_locations: HashMap<String, LocationPoint>,
}

pub struct EssentialsState {
    pub data_folder: String,
    pub store: EssentialsStore,
    pub prng_seed: u64,
}

impl EssentialsState {
    pub fn save(&self) {
        let path = format!("{}/essentials_store.json", self.data_folder);
        if let Ok(json) = serde_json::to_string_pretty(&self.store) {
            let _ = fs::write(&path, json);
        }
    }
}

static STATE: OnceLock<Mutex<EssentialsState>> = OnceLock::new();

fn with_state<R>(f: impl FnOnce(&mut EssentialsState) -> R) -> R {
    let mutex = STATE.get_or_init(|| {
        Mutex::new(EssentialsState {
            data_folder: ".".to_string(),
            store: EssentialsStore::default(),
            prng_seed: 0x9e37_79b9_7f4a_7c15,
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

fn init_state(data_folder: String) {
    let _ = fs::create_dir_all(&data_folder);
    let path = format!("{data_folder}/essentials_store.json");
    let store = match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<EssentialsStore>(&raw).unwrap_or_default(),
        Err(_) => EssentialsStore {
            homes: HashMap::new(),
            warps: HashMap::new(),
            spawn: Some(LocationPoint {
                world: "minecraft:overworld".to_string(),
                x: 198.5,
                y: 65.0,
                z: 159.5,
                yaw: 0.0,
                pitch: 0.0,
            }),
            last_locations: HashMap::new(),
        },
    };

    let mutex = STATE.get_or_init(|| {
        Mutex::new(EssentialsState {
            data_folder: data_folder.clone(),
            store,
            prng_seed: 0x9e37_79b9_7f4a_7c15,
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    guard.data_folder = data_folder;
}

fn do_teleport(player: &Player, server: &Server, loc: &LocationPoint) -> bool {
    let world_opt = server.get_world_by_name(&loc.world).or_else(|| {
        let all = server.get_all_worlds();
        all.into_iter().next()
    });

    if let Some(w) = world_opt {
        player.teleport((loc.x, loc.y, loc.z), Some(loc.yaw), Some(loc.pitch), w);
        player.play_custom_sound("minecraft:entity.enderman.teleport", SoundCategory::Players, 1.0, 1.0);
        true
    } else {
        false
    }
}

// =============================================================================
// Commands
// =============================================================================

struct SetHomeCmd;
impl CommandHandler for SetHomeCmd {
    fn handle(&self, sender: CommandSender, _server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let pos = match sender.position() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cНе удалось определить вашу позицию!"));
                return Ok(0);
            }
        };

        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let home_name = raw.trim().split_whitespace().next().unwrap_or("default");

        let p_name = player.get_name();
        let loc = LocationPoint {
            world: "minecraft:overworld".to_string(),
            x: (pos.0 * 10.0).round() / 10.0,
            y: (pos.1 * 10.0).round() / 10.0,
            z: (pos.2 * 10.0).round() / 10.0,
            yaw: 0.0,
            pitch: 0.0,
        };

        with_state(|st| {
            let player_homes = st.store.homes.entry(p_name.clone()).or_default();
            player_homes.insert(home_name.to_string(), loc.clone());
            st.save();
        });

        player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Players, 1.0, 1.5);
        sender.send_system_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §aТочка дома §e{} §aуспешно установлена! §7({:.1}, {:.1}, {:.1})",
            home_name, loc.x, loc.y, loc.z
        )));
        Ok(0)
    }
}

struct HomeCmd;
impl CommandHandler for HomeCmd {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let home_name = raw.trim().split_whitespace().next().unwrap_or("default");
        let p_name = player.get_name();

        let target_loc = with_state(|st| {
            st.store.homes.get(&p_name).and_then(|h| h.get(home_name).cloned())
        });

        if let Some(loc) = target_loc {
            if let Some(pos) = sender.position() {
                with_state(|st| {
                    st.store.last_locations.insert(p_name.clone(), LocationPoint {
                        world: "minecraft:overworld".to_string(),
                        x: pos.0,
                        y: pos.1,
                        z: pos.2,
                        yaw: 0.0,
                        pitch: 0.0,
                    });
                });
            }

            if do_teleport(&player, &server, &loc) {
                sender.send_system_message(legacy(&format!(
                    "§6§l✦ KRONOS §8» §aТелепортация на точку дома §e{}§a!",
                    home_name
                )));
            } else {
                sender.send_error(legacy("§cОшибка: мир дома недоступен!"));
            }
        } else {
            sender.send_error(legacy(&format!(
                "§cУ вас нет точки дома с именем '§e{}§c'. Установите её: §e/sethome {}",
                home_name, home_name
            )));
        }

        Ok(0)
    }
}

struct HomesListCmd;
impl CommandHandler for HomesListCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let p_name = player.get_name();
        let homes = with_state(|st| {
            st.store.homes.get(&p_name).cloned().unwrap_or_default()
        });

        if homes.is_empty() {
            sender.send_system_message(legacy(
                "§6§l✦ KRONOS §8» §7У вас пока нет сохранённых домов. Создайте: §e/sethome",
            ));
        } else {
            sender.send_system_message(legacy("§8§m────────────────────────────────────────"));
            sender.send_system_message(legacy("§6§l✦ ВАШИ ТОЧКИ ДОМА:"));
            for (name, loc) in homes {
                sender.send_system_message(legacy(&format!(
                    "  §e• §f{} §8— §7X: §a{:.1} §7Y: §a{:.1} §7Z: §a{:.1}",
                    name, loc.x, loc.y, loc.z
                )));
            }
            sender.send_system_message(legacy("§8§m────────────────────────────────────────"));
        }

        Ok(0)
    }
}

struct DelHomeCmd;
impl CommandHandler for DelHomeCmd {
    fn handle(&self, sender: CommandSender, _server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let home_name = raw.trim().split_whitespace().next().unwrap_or("");
        if home_name.is_empty() {
            sender.send_error(legacy("§eИспользование: §b/delhome <название>"));
            return Ok(0);
        }

        let p_name = player.get_name();
        let removed = with_state(|st| {
            if let Some(player_homes) = st.store.homes.get_mut(&p_name) {
                let r = player_homes.remove(home_name).is_some();
                if r {
                    st.save();
                }
                r
            } else {
                false
            }
        });

        if removed {
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS §8» §cТочка дома '§e{}§c' успешно удалена.",
                home_name
            )));
        } else {
            sender.send_error(legacy(&format!("§cТочка дома '§e{}§c' не найдена!", home_name)));
        }

        Ok(0)
    }
}

struct SpawnCmd;
impl CommandHandler for SpawnCmd {
    fn handle(&self, sender: CommandSender, server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let p_name = player.get_name();
        if let Some(pos) = sender.position() {
            with_state(|st| {
                st.store.last_locations.insert(p_name, LocationPoint {
                    world: "minecraft:overworld".to_string(),
                    x: pos.0,
                    y: pos.1,
                    z: pos.2,
                    yaw: 0.0,
                    pitch: 0.0,
                });
            });
        }

        let spawn_loc = with_state(|st| {
            st.store.spawn.clone().unwrap_or(LocationPoint {
                world: "minecraft:overworld".to_string(),
                x: 198.5,
                y: 65.0,
                z: 159.5,
                yaw: 0.0,
                pitch: 0.0,
            })
        });

        if do_teleport(&player, &server, &spawn_loc) {
            sender.send_system_message(legacy("§6§l✦ KRONOS §8» §aДобро пожаловать на спавн сервера!"));
        } else {
            sender.send_error(legacy("§cОшибка: мир спавна недоступен!"));
        }

        Ok(0)
    }
}

struct SetSpawnCmd;
impl CommandHandler for SetSpawnCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let pos = match sender.position() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cНе удалось определить позицию!"));
                return Ok(0);
            }
        };

        let loc = LocationPoint {
            world: "minecraft:overworld".to_string(),
            x: (pos.0 * 10.0).round() / 10.0,
            y: (pos.1 * 10.0).round() / 10.0,
            z: (pos.2 * 10.0).round() / 10.0,
            yaw: 0.0,
            pitch: 0.0,
        };

        with_state(|st| {
            st.store.spawn = Some(loc.clone());
            st.save();
        });

        sender.send_system_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §aТочка спавна сервера обновлена: §e{:.1}, {:.1}, {:.1}",
            loc.x, loc.y, loc.z
        )));
        Ok(0)
    }
}

struct BackCmd;
impl CommandHandler for BackCmd {
    fn handle(&self, sender: CommandSender, server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let p_name = player.get_name();
        let last_loc = with_state(|st| st.store.last_locations.get(&p_name).cloned());

        if let Some(loc) = last_loc {
            if do_teleport(&player, &server, &loc) {
                sender.send_system_message(legacy("§6§l✦ KRONOS §8» §aВозврат на предыдущую локацию!"));
            } else {
                sender.send_error(legacy("§cНе удалось телепортироваться на прошлую точку!"));
            }
        } else {
            sender.send_error(legacy("§cУ вас нет предыдущей сохранённой локации!"));
        }

        Ok(0)
    }
}

struct RtpCmd;
impl CommandHandler for RtpCmd {
    fn handle(&self, sender: CommandSender, server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let p_name = player.get_name();
        if let Some(pos) = sender.position() {
            with_state(|st| {
                st.store.last_locations.insert(p_name, LocationPoint {
                    world: "minecraft:overworld".to_string(),
                    x: pos.0,
                    y: pos.1,
                    z: pos.2,
                    yaw: 0.0,
                    pitch: 0.0,
                });
            });
        }

        let (rx, rz) = with_state(|st| {
            let mut s = st.prng_seed;
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            st.prng_seed = s;

            let dist = 300.0 + (s % 1200) as f64;
            let angle = ((s % 360) as f64).to_radians();
            (198.0 + dist * angle.cos(), 159.0 + dist * angle.sin())
        });

        let rtp_loc = LocationPoint {
            world: "minecraft:overworld".to_string(),
            x: rx.round(),
            y: 90.0,
            z: rz.round(),
            yaw: 0.0,
            pitch: 0.0,
        };

        if do_teleport(&player, &server, &rtp_loc) {
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS §8» §aСлучайная телепортация §eRTP §aна §7({:.0}, {:.0})!",
                rtp_loc.x, rtp_loc.z
            )));
        } else {
            sender.send_error(legacy("§cОшибка телепортации RTP!"));
        }

        Ok(0)
    }
}

struct WarpCmd;
impl CommandHandler for WarpCmd {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let player = match sender.as_player() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cКоманда доступна только игрокам!"));
                return Ok(0);
            }
        };

        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let warp_name = raw.trim().split_whitespace().next().unwrap_or("");
        if warp_name.is_empty() {
            sender.send_error(legacy("§eИспользование: §b/warp <название> §7(список: /warps)"));
            return Ok(0);
        }

        let target_warp = with_state(|st| st.store.warps.get(warp_name).cloned());

        if let Some(loc) = target_warp {
            let p_name = player.get_name();
            if let Some(pos) = sender.position() {
                with_state(|st| {
                    st.store.last_locations.insert(p_name, LocationPoint {
                        world: "minecraft:overworld".to_string(),
                        x: pos.0,
                        y: pos.1,
                        z: pos.2,
                        yaw: 0.0,
                        pitch: 0.0,
                    });
                });
            }

            if do_teleport(&player, &server, &loc) {
                sender.send_system_message(legacy(&format!(
                    "§6§l✦ KRONOS §8» §aВы успешно телепортированы на варп §e{}§a!",
                    warp_name
                )));
            } else {
                sender.send_error(legacy("§cМир варпа недоступен!"));
            }
        } else {
            sender.send_error(legacy(&format!("§cВарп '§e{}§c' не существует! Список: §e/warps", warp_name)));
        }

        Ok(0)
    }
}

struct SetWarpCmd;
impl CommandHandler for SetWarpCmd {
    fn handle(&self, sender: CommandSender, _server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let pos = match sender.position() {
            Some(p) => p,
            None => {
                sender.send_error(legacy("§cНе удалось определить позицию!"));
                return Ok(0);
            }
        };

        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let warp_name = raw.trim().split_whitespace().next().unwrap_or("");
        if warp_name.is_empty() {
            sender.send_error(legacy("§eИспользование: §b/setwarp <название>"));
            return Ok(0);
        }

        let loc = LocationPoint {
            world: "minecraft:overworld".to_string(),
            x: (pos.0 * 10.0).round() / 10.0,
            y: (pos.1 * 10.0).round() / 10.0,
            z: (pos.2 * 10.0).round() / 10.0,
            yaw: 0.0,
            pitch: 0.0,
        };

        with_state(|st| {
            st.store.warps.insert(warp_name.to_string(), loc.clone());
            st.save();
        });

        sender.send_system_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §aВарп §e{} §aуспешно создан! §7({:.1}, {:.1}, {:.1})",
            warp_name, loc.x, loc.y, loc.z
        )));
        Ok(0)
    }
}

struct WarpsListCmd;
impl CommandHandler for WarpsListCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let warps = with_state(|st| st.store.warps.clone());

        if warps.is_empty() {
            sender.send_system_message(legacy("§6§l✦ KRONOS §8» §7На сервере пока нет созданных варпов."));
        } else {
            sender.send_system_message(legacy("§8§m────────────────────────────────────────"));
            sender.send_system_message(legacy("§6§l✦ ДОСТУПНЫЕ ВАРПЫ СЕРВЕРА:"));
            for (name, loc) in warps {
                sender.send_system_message(legacy(&format!(
                    "  §e• §b{} §8— §7X: §a{:.1} §7Y: §a{:.1} §7Z: §a{:.1}",
                    name, loc.x, loc.y, loc.z
                )));
            }
            sender.send_system_message(legacy("§8§m────────────────────────────────────────"));
        }

        Ok(0)
    }
}

struct DelWarpCmd;
impl CommandHandler for DelWarpCmd {
    fn handle(&self, sender: CommandSender, _server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let warp_name = raw.trim().split_whitespace().next().unwrap_or("");
        if warp_name.is_empty() {
            sender.send_error(legacy("§eИспользование: §b/delwarp <название>"));
            return Ok(0);
        }

        let removed = with_state(|st| {
            let r = st.store.warps.remove(warp_name).is_some();
            if r {
                st.save();
            }
            r
        });

        if removed {
            sender.send_system_message(legacy(&format!(
                "§6§l✦ KRONOS §8» §cВарп '§e{}§c' успешно удалён.",
                warp_name
            )));
        } else {
            sender.send_error(legacy(&format!("§cВарп '§e{}§c' не найден!", warp_name)));
        }

        Ok(0)
    }
}

// =============================================================================
// Plugin Lifecycle
// =============================================================================

pub struct KronosEssentialsPlugin;

impl Plugin for KronosEssentialsPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kronos-essentials".into(),
            version: "2.0.0".into(),
            authors: vec!["UncleBug".into()],
            description: "Homes, Warps, Spawn, Back, and RTP for KRONOS 26.3".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_state(context.get_data_folder());

        context.register_permission(&Permission {
            node: PERM_PLAYER.to_string(),
            description: "Access to player teleport commands".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        context.register_permission(&Permission {
            node: PERM_ADMIN.to_string(),
            description: "Access to admin warp and spawn management".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: vec![],
        })?;

        // /sethome
        let sethome_cmd = Command::new(&["sethome".into()], "Установить точку дома (/sethome [имя])")
            .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(SetHomeCmd))
            .execute(SetHomeCmd);
        context.register_command(sethome_cmd, PERM_PLAYER);

        // /home
        let home_cmd = Command::new(&["home".into(), "h".into()], "Телепортироваться домой (/home [имя])")
            .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(HomeCmd))
            .execute(HomeCmd);
        context.register_command(home_cmd, PERM_PLAYER);

        // /homes
        let homes_cmd = Command::new(&["homes".into()], "Список всех точек дома (/homes)")
            .execute(HomesListCmd);
        context.register_command(homes_cmd, PERM_PLAYER);

        // /delhome
        let delhome_cmd = Command::new(&["delhome".into()], "Удалить точку дома (/delhome <имя>)")
            .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(DelHomeCmd));
        context.register_command(delhome_cmd, PERM_PLAYER);

        // /spawn
        let spawn_cmd = Command::new(&["spawn".into()], "Телепортироваться на спавн сервера")
            .execute(SpawnCmd);
        context.register_command(spawn_cmd, PERM_PLAYER);

        // /setspawn
        let setspawn_cmd = Command::new(&["setspawn".into()], "Установить точку спавна сервера")
            .execute(SetSpawnCmd);
        context.register_command(setspawn_cmd, PERM_ADMIN);

        // /back
        let back_cmd = Command::new(&["back".into()], "Вернуться на предыдущую локацию")
            .execute(BackCmd);
        context.register_command(back_cmd, PERM_PLAYER);

        // /rtp
        let rtp_cmd = Command::new(&["rtp".into()], "Случайная телепортация в дикую местность")
            .execute(RtpCmd);
        context.register_command(rtp_cmd, PERM_PLAYER);

        // /warp
        let warp_cmd = Command::new(&["warp".into(), "w".into()], "Телепортироваться на варп (/warp <название>)")
            .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(WarpCmd));
        context.register_command(warp_cmd, PERM_PLAYER);

        // /warps
        let warps_cmd = Command::new(&["warps".into()], "Список всех варпов сервера")
            .execute(WarpsListCmd);
        context.register_command(warps_cmd, PERM_PLAYER);

        // /setwarp
        let setwarp_cmd = Command::new(&["setwarp".into()], "Установить точку варпа (/setwarp <название>)")
            .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(SetWarpCmd));
        context.register_command(setwarp_cmd, PERM_ADMIN);

        // /delwarp
        let delwarp_cmd = Command::new(&["delwarp".into()], "Удалить варп (/delwarp <название>)")
            .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(DelWarpCmd));
        context.register_command(delwarp_cmd, PERM_ADMIN);

        tracing::info!("KRONOS Essentials v2.0.0 initialized successfully!");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        with_state(|st| st.save());
        tracing::info!("KRONOS Essentials unloaded.");
        Ok(())
    }
}

register_plugin!(KronosEssentialsPlugin);
