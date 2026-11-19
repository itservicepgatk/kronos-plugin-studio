# KRONOS Essentials v2.0.0 — Rust WASM Plugin (Pumpkin 26.3)

Модульный плагин телепортации, общих домов, безопасного `/rtp`, Anti-Combat PvP-таймера и интерактивных меню на официальном **WASI Preview 2 Component Model (`pumpkin-plugin-api = "0.1.0"`)** для серверов **Pumpkin 26.3** (Java 1.20.1–26.3 + Bedrock).

## Ключевые возможности
- **100% официальный `pumpkin_plugin_api` (без вымышленного `pumpkin_host` FFI):** регистрация через `register_plugin!`, права с пространством имён `kronos-essentials:player` и `kronos-essentials:admin`, нулевые ошибки линковки в Wasmtime.
- **Безопасный `/rtp`:** вместо захардкоженной высоты `Y = 90.0` выполняется сканирование реальной поверхности (`get_motion_blocking_height`, `get_top_block_y`, `get_block_state`, проверка границы мира `get_border().contains(...)`) с фильтрацией лавы, воды, листвы, кактусов и пустоты + защита `Resistance` и `SlowFalling`.
- **Общие дома (`/sharehome`, `/home share`, `/home unshare`):** делитесь домами с друзьями, просматривайте список доступных домов и отзывайте доступ в один клик. Все связи сохраняются атомарно в `essentials_store.json` в папке данных плагина (`context.get_data_folder()`).
- **Интерактивное меню (`/menu`, `/homes`, `/warps`, `/sharehome`):**
  - Для **Java Edition (1.20.1–26.3)**: кликабельное чат-меню на `TextComponent` (`click_run_command`, `click_suggest_command`, `hover_show_text` с координатами и иконками, удаление и переключение ★ в один клик).
  - Для **Bedrock Edition**: нативная форма `SimpleForm` через `pumpkin_plugin_api::wit::pumpkin::plugin::forms`.
- **Anti-Combat (15-секундный PvP-таймер):** подписка на `EntityDamageByEntityEvent` и `EntityDamageEvent` блокирует `/spawn`, `/home`, `/warp`, `/rtp`, `/back`, `/tpa`, `/tpahere`, `/tpaccept` с выводом в чат и ActionBar: `§c⚔ Вы в бою! Телепортация заблокирована на {N}с`.
- **Мультиверсионные звуки и эффекты (1.20.1–26.3):** исключительно `player.play_custom_sound(..., SoundCategory::Players, ...)` с безопасными ID, `StatusEffectType::{Blindness, Resistance, SlowFalling}`, частицы `Portal` / `EndRod` и Title/ActionBar.

## Команды
| Команда | Описание | Право |
| :--- | :--- | :--- |
| `/menu`, `/homes` | Открыть интерактивный хаб домов и быстрых телепортов | `kronos-essentials:player` |
| `/sethome [имя]` | Создать или обновить точку дома | `kronos-essentials:player` |
| `/home [имя \| владелец:дом]` | Телепорт в свой или общий дом | `kronos-essentials:player` |
| `/home fav <дом>` | Назначить главный дом ★ | `kronos-essentials:player` |
| `/sharehome <дом> <друг>` / `/home share <дом> <друг>` | Открыть другу доступ к дому | `kronos-essentials:player` |
| `/home unshare <дом> <друг>` / `/sharehome remove <дом> <друг>` | Отозвать доступ друга | `kronos-essentials:player` |
| `/delhome <имя>` | Удалить точку дома | `kronos-essentials:player` |
| `/rtp` | Безопасная случайная телепортация | `kronos-essentials:player` |
| `/spawn`, `/back` | Телепорт на спавн / возврат на предыдущую позицию или точку смерти | `kronos-essentials:player` |
| `/warp <имя>`, `/warps` | Телепорт на варп / интерактивный список варпов | `kronos-essentials:player` |
| `/tpa <ник>`, `/tpahere <ник>`, `/tpaccept`, `/tpdeny` | Запросы на телепортацию к игрокам | `kronos-essentials:player` |
| `/setspawn`, `/setwarp <имя> [описание]`, `/delwarp <имя>` | Админ-управление спавном и варпами | `kronos-essentials:admin` |

## Сборка для Pumpkin 26.3
```bash
cargo test
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --release
```
Готовый файл `target/wasm32-wasip2/release/kronos_essentials.wasm` скопируйте в папку `plugins/` вашего сервера Pumpkin.
