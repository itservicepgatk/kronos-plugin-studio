# 📋 Master Prompt for Google AI Studio (`aistudio.google.com/apps`)

Скопируй текст ниже и вставь его в поле ввода на **[aistudio.google.com/u/1/apps](https://aistudio.google.com/u/1/apps)**, выбрав **Max mode** (модель Gemini 4 Argon или Hydrogen):

---

```markdown
Create a complete, modern web application named "KRONOS Plugin Studio" that generates and edits WebAssembly (WASM) Rust plugins for the Pumpkin Minecraft Server (Protocol 763-777, Java 1.20.1–26.3 + Bedrock).

### Core Requirements & Architecture:

1. **AI Generation with Gemini API:**
   - Use Google AI Studio models (Gemini 4 Argon with deep reasoning, Gemini 4 Hydrogen, Gemini 2.5 Pro/Flash).
   - Allow user to provide their Gemini API key (stored in localStorage) or use the environment API key.
   - Model switcher and "Max Mode" toggle (adjusts thinking budget and temperature for complex Rust algorithms).
   - System prompt must be pre-loaded with the official Pumpkin Rust WASM Plugin specification (WASI Preview 2 Component Model):
     - **Compilation Target**: `wasm32-wasip2` (Wasm Component Model). DO NOT use legacy wasip1 or raw C-FFI `extern "C" fn init()`.
     - **API Library**: `pumpkin-plugin-api = "0.1.0"` (from https://github.com/UncleBugDev/kronos-plugins).
     - **Entrypoint**: Implement `pumpkin_plugin_api::Plugin` trait and register via `register_plugin!(MyPlugin);`.
     - **Commands**: Build trees with `pumpkin_plugin_api::command::Command` and execute with `CommandHandler`.
     - **Permissions**: Every permission node MUST start with the plugin name from metadata, e.g. `"kronos-essentials:player"`, `"auth-plugin:login"`.
     - **Multi-Version Safety**:
       - Sounds: ONLY `player.play_custom_sound("minecraft:entity.experience_orb.pickup", ...)` with standard 1.20.1 sound IDs (never raw `player.play_sound`).
       - Texts: `TextComponent::from_legacy_string("§6...")`.
       - Thread Safety: NEVER hold a `Mutex` lock across host WIT calls (`player.send_message`, `player.open_gui`).
     - **Persistence**: Store config and player data in `context.get_data_folder()`.

2. **Project File System & Workspace:**
   - In-memory virtual file system storing multiple files (e.g., `Cargo.toml`, `src/lib.rs`, `README.md`, `data/config.json`).
   - Left sidebar with file tree, file creation, file deletion, and target build stats (`wasm32-wasip2`).
   - Center code editor with line numbers, syntax styling, tab bar, copy button, and character/line counter.

3. **Round-Trip ZIP Export & Import (Crucial):**
   - **Download .ZIP**: Uses JSZip and FileSaver to package all project files into a `.zip` file (e.g. `kronos-essentials.zip`) that can be downloaded with one click.
   - **Open / Resume .ZIP**: Drag-and-drop zone or "Open .ZIP" button where the user can upload ANY previously downloaded `.zip` file. The app unzips it, extracts `Cargo.toml` and all `src/*.rs` files, loads them into the editor, and prompts the AI: "Loaded existing plugin project. Let's continue working on it!".

4. **Interactive AI Assistant Panel:**
   - Right-side chat interface displaying conversation history.
   - Quick prompt chips: "Add /spawn command", "Add VIP rank prefix to tab list", "Add password hashing", "Create GUI menu".
   - When Gemini generates or modifies code, automatically parse markdown code blocks (` ```rust:src/lib.rs `).
   - Show a prominent "Apply All to Workspace" button so the user can update the project with a single click without manual copying.

5. **Built-in Starter Templates (WASI Preview 2):**
   - Modal with pre-configured templates using `pumpkin-plugin-api`:
     - KRONOS Essentials (Homes, Spawns, Warps, RTP, Back)
     - KRONOS Ranks (Prefixes, Tab list, Suffixes, Permissions)
     - KRONOS Guard (Anti-cheat, Attack speeds, WorldGuard protections)
     - KRONOS Regions (Claims, Wand selection, Flags)
     - Minimal Blank Plugin (with `register_plugin!`)

6. **Design & UX:**
   - Dark modern theme inspired by JetBrains / VS Code / Minecraft palette (`#0f0f12`, amber accents `#f59e0b`, rust accents `#ea580c`, emerald status `#10b981`).
   - Clean responsive 3-column layout (File Tree | Code Editor | AI Chat).
   - Responsive and fluid with Lucide icons.
```
