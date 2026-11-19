import { PluginTemplate } from '../types';
import { KRONOS_ESSENTIALS_TEMPLATE } from './kronosEssentialsTemplate';
import { KRONOS_GUARD_TEMPLATE } from './kronosGuardTemplate';
import { KRONOS_RANKS_TEMPLATE } from './kronosRanksTemplate';
import { KRONOS_REGIONS_TEMPLATE } from './kronosRegionsTemplate';

export const STARTER_TEMPLATES: PluginTemplate[] = [
  KRONOS_ESSENTIALS_TEMPLATE,
  KRONOS_RANKS_TEMPLATE,
  KRONOS_GUARD_TEMPLATE,
  KRONOS_REGIONS_TEMPLATE,
  {
    id: 'blank-plugin',
    title: 'Minimal Starter Plugin',
    description: 'Clean WASI Preview 2 boilerplate with pumpkin_plugin_api, register_plugin!, permissions, and command handler.',
    tags: ['Basic', 'Boilerplate', 'wasm32-wasip2'],
    files: {
      'Cargo.toml': `[package]
name = "custom-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
pumpkin-plugin-api = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
`,
      'src/lib.rs': `use pumpkin_plugin_api::{
    Context, Plugin, PluginMetadata, Server,
    command::{ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType},
    commands::CommandHandler,
    events::{EventHandler, EventPriority, PlayerJoinEvent},
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    text::TextComponent,
    wit::pumpkin::plugin::event::PlayerJoinEventData,
};

const PERM_USE: &str = "custom-plugin:use";

struct HelloCmd;
impl CommandHandler for HelloCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        sender.send_message(TextComponent::from_legacy_string("§6[KRONOS] §aПлагин работает успешно!"));
        Ok(0)
    }
}

struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, _server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let player = &event.player;
        player.send_system_message(TextComponent::from_legacy_string("§aДобро пожаловать на сервер!"), false);
        player.play_custom_sound(
            "minecraft:entity.experience_orb.pickup",
            pumpkin_plugin_api::wit::pumpkin::plugin::sound::SoundCategory::Master,
            1.0,
            1.0,
        );
        event
    }
}

pub struct CustomPlugin;

impl Plugin for CustomPlugin {
    fn new() -> Self { Self }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "custom-plugin".into(),
            version: "0.1.0".into(),
            authors: vec!["UncleBug".into()],
            description: "Custom plugin for KRONOS Minecraft server".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        context.register_permission(&Permission {
            node: PERM_USE.to_string(),
            description: "Use custom-plugin command".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        let cmd = Command::new(&["custom".into(), "cp".into()], "Custom plugin command")
            .execute(HelloCmd);
        context.register_command(cmd, PERM_USE);

        context.register_event_handler::<PlayerJoinEvent, _>(
            JoinHandler,
            EventPriority::Normal,
            false,
        )?;

        tracing::info!("CustomPlugin initialized successfully!");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        tracing::info!("CustomPlugin unloaded.");
        Ok(())
    }
}

register_plugin!(CustomPlugin);
`,
      'README.md': `# Custom Plugin for KRONOS / Pumpkin

A Rust WASM plugin compiled for target \`wasm32-wasip2\` (WASI Preview 2 Component Model).

### Build instructions:
\`\`\`bash
cargo build --target wasm32-wasip2 --release
\`\`\`
Drop target \`.wasm\` file into \`/home/unclebug/pumpkin-server/plugins/\`.
`
    }
  }
];
