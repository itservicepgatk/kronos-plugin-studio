# KRONOS Economy v1.0.0 (`kronos-economy`)

Полноценная экономическая система для Minecraft-сервера на ядре **Pumpkin 26.3** (`pumpkin-plugin-api = "0.1.0"`, Rust, WASI Preview 2 / `wasm32-wasip2`).

## Структура плагина

```text
kronos-economy/
├── Cargo.toml        # Зависимости и конфигурация cdylib для wasm32-wasip2
├── README.md         # Документация плагина и инструкция по сборке
└── src/
    ├── lib.rs        # Точка входа (Plugin + register_plugin!), события и юнит-тесты
    ├── models.rs     # Модель счетов, fixed-point u64 арифметика, парсер и транзакции
    ├── storage.rs    # Потокобезопасный State и атомарное сохранение economy.json
    ├── menu.rs       # Двойной UI (Java TextComponent + Bedrock SimpleForm) и звуки 1.20.1
    └── commands.rs   # Команды /money (/bal), /pay, /baltop и /eco
```

## Реализованный функционал

- **Точная арифметика `u64` (fixed-point, `SCALE = 100`):** 1 монета = 100 сотых. Поддерживается ввод целых и дробных сумм (`100`, `12.50`, `12,50`) без ошибок округления `f64`, с защитой от переполнения через `checked_add` / `checked_sub`.
- **Защита от дюпа в многопотоке:** все проверки (овердрафт, перевод самому себе, ноль/отрицательные значения, переполнение `u64`) и обновление обоих счетов выполняются атомарно под одним `Mutex`. Запись на диск и хост-вызовы выполняются строго после освобождения лока.
- **Атомарное хранилище:** балансы сохраняются в `context.get_data_folder() / "economy.json"` через временный файл `.json.tmp` и `fs::rename`.
- **Двойной UI (Java + Bedrock):**
  - Java Edition: кликабельные `TextComponent`-меню с hover-подсказками (цепочечные вызовы `chainable_methods`).
  - Bedrock Edition: нативные графические окна `SimpleForm` через `player.as_bedrock()`.
- **Звуки (1.20.1+):**
  - Успешный перевод / операция: `minecraft:entity.experience_orb.pickup`
  - Ошибка / отказ: `minecraft:entity.villager.no`
  - Навигация по меню: `minecraft:ui.button.click`

## Команды и права

| Команда | Алиасы | Право | Описание |
| :--- | :--- | :--- | :--- |
| `/money [игрок]` | `/bal`, `/balance` | `kronos-economy:player` (`Allow`) | Открывает карточку баланса (Java/Bedrock UI) |
| `/pay <игрок> <сумма>` | — | `kronos-economy:player` (`Allow`) | Безопасный перевод монет с защитой от овердрафта, отрицательных сумм и перевода себе |
| `/baltop` | `/moneytop` | `kronos-economy:player` (`Allow`) | Топ-10 богачей сервера на `TextComponent` с hover-подсказками и долей экономики |
| `/eco <give\|take\|set> <игрок> <сумма>` | — | `kronos-economy:admin` (`Op(3)`) | Административное управление балансами игроков (доступно и из консоли) |

## Сборка и тестирование

1. Поместите папку `kronos-economy` в `examples/kronos-economy` внутри `pumpkin-plugin-dev-kit`.
2. Добавьте `"examples/kronos-economy"` в массив `members` корневого `Cargo.toml`.
3. Выполните команды из корня workspace:

```bash
# Прогон всех юнит-тестов (включая многопоточный тест анти-дюпа и атомарную запись)
cargo test -p kronos-economy

# Сборка релизного .wasm плагина
cargo build --release -p kronos-economy --target wasm32-wasip2
```

Готовый файл `target/wasm32-wasip2/release/kronos_economy.wasm` скопируйте в папку `plugins/` сервера Pumpkin.
