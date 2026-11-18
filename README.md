# 🦀 KRONOS Plugin Studio

An AI-powered Web Application for developing, generating, and iterating on **Rust WebAssembly (WASM)** plugins for the **KRONOS / Pumpkin 26.3 Minecraft Server** (compatible with Minecraft Java 1.20.1–26.3 and Bedrock).

Powered by **Google AI Studio** models (**Gemini 4 Argon** with Max Mode deep reasoning & **Gemini 4 Hydrogen** for high-speed coding).

---

## 🌟 Key Features

1. **AI-Driven Plugin Generation**:
   - Integrated with Google AI Studio Gemini API (`gemini-4-argon`, `gemini-4-hydrogen`, `gemini-2.5-pro`, `gemini-2.5-flash`).
   - Deep Reasoning **Max Mode** for complex architecture, state synchronization, and thread-safe data structures.
   - Built-in knowledge base of Pumpkin WASM API, hooks (`on_player_join`, `on_player_chat`, `on_command`, `on_block_break`, `on_block_place`), permissions (`fs.read.data`, `fs.write.data`), and `wasm32-wasip1` standards.

2. **One-Click File Application**:
   - When Gemini generates or updates code, click **"Apply All to Workspace"** to update all project files instantaneously.

3. **Round-Trip .ZIP Export & Import**:
   - **Download .ZIP**: Bundles your plugin (`Cargo.toml`, `src/lib.rs`, `README.md`, configurations) into a clean, ready-to-compile archive.
   - **Open / Resume .ZIP**: Drag-and-drop any previously downloaded `.zip` file into the browser window or click "Open .ZIP" to unpack all files and continue prompt-driven iteration.

4. **Built-In Starter Templates**:
   - **Minimal Starter**: Clean lifecycle boilerplate with logging and build configs.
   - **Auth & Security Plugin**: Player registration (`/register`), login (`/login`), password hashing, and movement locks.
   - **Tab & Ranks Plugin**: Tablist header/footer, prefix/suffix formatting, and permissions-based rank tiers (Owner, Admin, VIP, Player).

---

## 🚀 Quick Start (Local Run)

### 1. Install Dependencies
\`\`\`bash
npm install
\`\`\`

### 2. Run Development Server
\`\`\`bash
npm run dev
\`\`\`
Open [http://localhost:3000](http://localhost:3000) in your browser.

### 3. Set API Key
Click **"Set API Key"** in the top navigation bar and enter your Google AI Studio API key (stored securely in browser `localStorage`). Get a key for free at: [https://aistudio.google.com/app/apikey](https://aistudio.google.com/app/apikey).

---

## ⚡ How to Build & Deploy Generated Plugins

Once you download your plugin's `.zip` file from the app:

1. **Unzip the plugin on your build machine or server**:
   \`\`\`bash
   unzip my-plugin.zip -d my-plugin
   cd my-plugin
   \`\`\`

2. **Compile to WebAssembly (`wasm32-wasip1`)**:
   \`\`\`bash
   cargo build --target wasm32-wasip1 --release
   \`\`\`

3. **Deploy to KRONOS Server**:
   Copy the resulting `.wasm` binary into the Pumpkin server's plugins directory:
   \`\`\`bash
   cp target/wasm32-wasip1/release/my_plugin.wasm /home/unclebug/pumpkin-server/plugins/
   \`\`\`
   Pumpkin will automatically hot-reload or load the plugin on restart!

---

## 📄 License
MIT License. Built for the KRONOS Server Team.
