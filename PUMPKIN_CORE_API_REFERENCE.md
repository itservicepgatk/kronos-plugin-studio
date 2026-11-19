# 🎃 Pumpkin Minecraft Core & Plugin API Reference (Protocol 763–777 / 1.20.1–26.3)

> **Полный справочник интерфейсов ядра Pumpkin (Rust Core) и WASI Preview 2 Component Model (`wasm32-wasip2`).**
> Предназначен для разработчиков и нейросетей (Gemini 4 Argon/Hydrogen, Claude, GPT, Cursor) для создания на 100% совместимых серверных плагинов.

---

## 1. Архитектура и среда выполнения

| Параметр | Значение | Примечание |
| :--- | :--- | :--- |
| **Ядро сервера** | **Pumpkin** (чистый Rust) | Сверхвысокая производительность, многопоточность |
| **Протокол** | `763–777` | Клиенты Java 1.20.1 – 26.3 + Bedrock через Velocity/Geyser |
| **WASM Target** | `wasm32-wasip2` | **WASI Preview 2 Component Model** (НЕ wasip1!) |
| **ABI / Интерфейсы** | WebAssembly Interface Types (**WIT**) | `wit/` каталог в корне этого репозитория |
| **Основной крейт** | `pumpkin-plugin-api = "0.1.0"` | Обёртки и макрос `register_plugin!` |
| **Песочница** | Wasmtime inside Pumpkin host | Строгая изоляция файловой системы и памяти |

---

## 2. Структура `Cargo.toml` плагина

```toml
[package]
name = "my-plugin"
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
```

---

## 3. Точка входа: Трейт `Plugin` и макрос регистрации

Каждый плагин обязан реализовывать трейт `Plugin` и вызывать `register_plugin!(MyPlugin);`:

```rust
use pumpkin_plugin_api::{
    register_plugin, Context, Plugin, PluginMetadata,
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
};

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn new() -> Self { Self }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".into(),
            version: "0.1.0".into(),
            authors: vec!["UncleBug".into()],
            description: "Описание функционала плагина".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        // Регистрация прав, команд, слушателей событий, тасок
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        // Сохранение несохраненных данных
        Ok(())
    }
}

register_plugin!(MyPlugin);
```

---

## 4. Справочник методов ядра

### 4.1. `Context` (Доступен в `on_load`)
Контекст плагина отвечает за регистрацию всех сущностей в ядре сервера:

| Метод | Сигнатура | Назначение |
| :--- | :--- | :--- |
| `get_data_folder()` | `fn get_data_folder(&self) -> String` | Путь к изолированной папке данных плагина в песочнице |
| `register_permission(&perm)` | `fn register_permission(&self, perm: &Permission) -> Result<(), String>` | Регистрация права (ОБЯЗАТЕЛЬНО: `my-plugin:perm`) |
| `register_command(cmd, perm)` | `fn register_command(&self, cmd: Command, perm: &str)` | Регистрация команды с привязкой к праву |
| `register_event_handler(h, prio, b)` | `fn register_event_handler<E, H>(&self, h: H, prio: EventPriority, blocking: bool) -> Result<(), String>` | Подписка на события сервера |
| `schedule_repeating_task(delay, period, task)` | `fn schedule_repeating_task(&self, delay: u64, period: u64, task: fn(Server))` | Фоновая периодическая задача (в тиках) |
| `schedule_delayed_task(delay, task)` | `fn schedule_delayed_task(&self, delay: u64, task: fn(Server))` | Отложенная задача (в тиках) |

---

### 4.2. `Player` (Управление игроком)
Все методы над игроком выполняются через ресурс WIT-хоста:

| Метод | Сигнатура | Назначение |
| :--- | :--- | :--- |
| `get_name()` | `fn get_name(&self) -> String` | Имя (никнейм) игрока |
| `get_id()` | `fn get_id(&self) -> String` / UUID | Уникальный UUID игрока |
| `get_ping()` | `fn get_ping(&self) -> u32` | Пинг игрока в миллисекундах |
| `get_position()` | `fn get_position(&self) -> (f64, f64, f64)` | Текущие координаты X, Y, Z |
| `get_yaw()` / `get_pitch()` | `fn get_yaw(&self) -> f32` | Направление взгляда |
| `get_world()` | `fn get_world(&self) -> World` | Мир, в котором находится игрок |
| `get_gamemode()` | `fn get_gamemode(&self) -> GameMode` | Текущий игровой режим (Survival, Creative и т.д.) |
| `set_gamemode(mode)` | `fn set_gamemode(&self, mode: GameMode) -> bool` | Смена игрового режима |
| `has_permission(node)` | `fn has_permission(&self, node: &str) -> bool` | Проверка права у игрока |
| `set_permission(node, val)` | `fn set_permission(&self, node: &str, val: bool)` | Выдать/отозвать персональное право |
| `send_system_message(text, overlay)` | `fn send_system_message(&self, text: TextComponent, overlay: bool)` | Сообщение в чат (`overlay=false`) или ActionBar (`overlay=true`) |
| `show_title(text)` | `fn show_title(&self, text: TextComponent)` | Показать главный заголовок (Title) на экране |
| `show_subtitle(text)` | `fn show_subtitle(&self, text: TextComponent)` | Показать подзаголовок (Subtitle) |
| `show_actionbar(text)` | `fn show_actionbar(&self, text: TextComponent)` | Показать текст над инвентарем (ActionBar) |
| `send_title_animation(in, stay, out)` | `fn send_title_animation(&self, in: i32, stay: i32, out: i32)` | Настройка таймингов анимации Title в тиках |
| `teleport(x, y, z, yaw, pitch)` | `fn teleport(&self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32)` | Мгновенный перенос игрока на координаты |
| `play_custom_sound(name, cat, vol, pitch)` | `fn play_custom_sound(&self, name: &str, cat: SoundCategory, vol: f32, pitch: f32)` | Воспроизвести звук по имени (`"minecraft:..."`) |
| `set_tab_list_header_footer(h, f)` | `fn set_tab_list_header_footer(&self, h: TextComponent, f: TextComponent)` | Установить верхний и нижний колонтитул Tab-листа |
| `set_tab_list_name(name)` | `fn set_tab_list_name(&self, name: Option<TextComponent>)` | Префикс/никнейм игрока в Tab-листе |
| `open_gui(gui)` | `fn open_gui(&self, gui: Gui)` | Открыть инвентарное меню (Chest GUI) |
| `close_container()` | `fn close_container(&self)` | Принудительно закрыть открытое меню/сундук |
| `kick(reason)` | `fn kick(&self, reason: TextComponent)` | Кикнуть игрока с сервера с сообщением |

---

### 4.3. `Server` (Управление сервером)

| Метод | Сигнатура | Назначение |
| :--- | :--- | :--- |
| `get_player_count()` | `fn get_player_count(&self) -> u32` | Количество игроков онлайн |
| `get_players()` | `fn get_players(&self) -> Vec<Player>` | Список всех активных игроков онлайн |
| `get_player_by_name(name)` | `fn get_player_by_name(&self, name: &str) -> Option<Player>` | Поиск игрока по никнейму |
| `get_player_by_id(uuid)` | `fn get_player_by_id(&self, uuid: &str) -> Option<Player>` | Поиск игрока по UUID |
| `broadcast_message(text)` | `fn broadcast_message(&self, text: TextComponent)` | Отправить сообщение всем игрокам на сервере |
| `execute_command(command)` | `fn execute_command(&self, command: &str) -> bool` | Выполнить команду от имени консоли сервера |

---

## 5. Система команд (`Command` & `CommandNode`)

Команды регистрируются в виде дерева аргументов:

```rust
use pumpkin_plugin_api::{
    command::{ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType},
    commands::CommandHandler, Server, text::TextComponent,
    wit::pumpkin::plugin::command::Arg,
};

struct WarpCmd;
impl CommandHandler for WarpCmd {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let warp_name = match args.get_value("name") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };

        if warp_name.is_empty() {
            sender.send_message(TextComponent::from_legacy_string("§6Использование: §e/warp <название>"));
            return Ok(0);
        }

        sender.send_message(TextComponent::from_legacy_string(&format!("§aТелепортация на варп §e{warp_name}§a...")));
        Ok(0)
    }
}

// В on_load():
let cmd = Command::new(&["warp".into(), "w".into()], "Телепортация на серверный варп")
    .execute(WarpCmd)
    .then(
        CommandNode::argument("name", &ArgumentType::String(StringType::Word))
            .execute(WarpCmd)
    );

context.register_command(cmd, "kronos-essentials:player");
```

---

## 6. Система событий (Events)

События перехватывают действия в игре и могут отменять их или модифицировать:

```rust
use pumpkin_plugin_api::events::{EventHandler, EventPriority, PlayerChatEvent};
use pumpkin_plugin_api::wit::pumpkin::plugin::event::PlayerChatEventData;

struct ChatListener;
impl EventHandler<PlayerChatEvent> for ChatListener {
    fn handle(&self, _server: Server, mut event: PlayerChatEventData) -> PlayerChatEventData {
        // Добавление красивого префикса или фильтрация мата:
        event.message = format!("§7{} §8» §f{}", event.player.get_name(), event.message);
        event
    }
}

// В on_load():
context.register_event_handler::<PlayerChatEvent, _>(ChatListener, EventPriority::Normal, false)?;
```

Доступные типы событий в `pumpkin_plugin_api::events`:
- `PlayerJoinEvent`
- `PlayerLeaveEvent`
- `PlayerRespawnEvent`
- `PlayerChatEvent`
- `PlayerCommandEvent`
- `BlockPlaceEvent`
- `BlockBreakEvent`
- `InventoryClickEvent`
- `InventoryCloseEvent`
- `ServerLoadEvent`

---

## 7. Чеклист безопасности перед компиляцией

1. [ ] **Target**: строго `wasm32-wasip2`.
2. [ ] **Звуки**: только `player.play_custom_sound("minecraft:...", ...)` с ID из 1.20.1 (никаких `player.play_sound`).
3. [ ] **Потоки**: не удерживать блокировку `Mutex` во время вызова `player.*` или `server.*`.
4. [ ] **Права**: префикс права обязан совпадать с именем плагина (`plugin-name:permission`).
5. [ ] **Текст**: использовать `TextComponent::from_legacy_string("§a...")`.
