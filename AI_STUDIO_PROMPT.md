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
   - System prompt must be pre-loaded with the Pumpkin Rust WASM Plugin specification:
     - Target: `wasm32-wasip1`
     - Crate type: `cdylib` in `Cargo.toml`
     - Host hooks: `init() -> i32`, `cleanup()`, `on_player_join(player_id, name)`, `on_player_leave(player_id)`, `on_player_chat(player_id, message) -> Option<String>`, `on_player_command(player_id, command) -> bool`, `on_block_break`, `on_block_place`
     - Permissions: `fs.read.data`, `fs.write.data`
     - Thread-safe storage with mutexes and JSON persistence in `data/<plugin_name>/`.

2. **Project File System & Workspace:**
   - In-memory virtual file system storing multiple files (e.g., `Cargo.toml`, `src/lib.rs`, `README.md`, `data/config.json`).
   - Left sidebar with file tree, file creation, file deletion, and target build stats (`wasm32-wasip1`).
   - Center code editor with line numbers, syntax styling, tab bar, copy button, and character/line counter.

3. **Round-Trip ZIP Export & Import (Crucial):**
   - **Download .ZIP**: Uses JSZip and FileSaver to package all project files into a `.zip` file (e.g. `auth-plugin.zip`) that can be downloaded with one click.
   - **Open / Resume .ZIP**: Drag-and-drop zone or "Open .ZIP" button where the user can upload ANY previously downloaded `.zip` file. The app unzips it, extracts `Cargo.toml` and all `src/*.rs` files, loads them into the editor, and prompts the AI: "Loaded existing plugin project. Let's continue working on it!".

4. **Interactive AI Assistant Panel:**
   - Right-side chat interface displaying conversation history.
   - Quick prompt chips: "Add /spawn command", "Add VIP rank prefix to tab list", "Add password hashing", "Create GUI menu".
   - When Gemini generates or modifies code, automatically parse markdown code blocks (` ```rust:src/lib.rs `).
   - Show a prominent "Apply All to Workspace" button so the user can update the project with a single click without manual copying.

5. **Built-in Starter Templates:**
   - Modal with pre-configured templates:
     - Minimal Starter Plugin
     - Auth & Security Plugin (registration, login, hashing)
     - Tab & Ranks Plugin (colored prefixes, header/footer)
     - Menu & GUI Plugin (chest inventory menus)

6. **Design & UX:**
   - Dark modern theme inspired by JetBrains / VS Code / Minecraft palette (`#0f0f12`, amber accents `#f59e0b`, rust accents `#ea580c`, emerald status `#10b981`).
   - Clean responsive 3-column layout (File Tree | Code Editor | AI Chat).
   - Responsive and fluid with Lucide icons.
```
