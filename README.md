# 🦀 KRONOS Plugin Studio

An AI-powered Web Application for developing, generating, and iterating on **Rust WebAssembly (WASM)** plugins for the **KRONOS / Pumpkin 26.3 Minecraft Server** (compatible with Minecraft Java 1.20.1–26.3 and Bedrock).

Powered by **Google AI Studio** models (**Gemini 4 Argon** with Max Mode deep reasoning & **Gemini 4 Hydrogen** for high-speed coding).

---

## 📚 Essential Developer Links
- **Official Plugin Templates & Core**: [https://github.com/UncleBugDev/kronos-plugins](https://github.com/UncleBugDev/kronos-plugins)
- **AI Plugin Developer Guide**: [https://github.com/UncleBugDev/kronos-plugins/blob/main/AI_PLUGIN_DEV_GUIDE.md](https://github.com/UncleBugDev/kronos-plugins/blob/main/AI_PLUGIN_DEV_GUIDE.md)

---

## 🌟 Key Features

1. **AI-Driven Plugin Generation**:
   - Integrated with Google AI Studio Gemini API (`gemini-4-argon`, `gemini-4-hydrogen`, `gemini-2.5-pro`, `gemini-2.5-flash`).
   - Deep Reasoning **Max Mode** for complex architecture, state synchronization, and thread-safe data structures.
   - Built-in knowledge base of Pumpkin WASM API (**WASI Preview 2 Component Model**, target `wasm32-wasip2`), `pumpkin-plugin-api`, commands, events, permissions, and multi-version safety (Java 1.20.1–26.3).

2. **One-Click File Application**:
   - When Gemini generates or updates code, click **"Apply All to Workspace"** to update all project files instantaneously.

3. **Round-Trip .ZIP Export & Import**:
   - **Download .ZIP**: Bundles your plugin (`Cargo.toml`, `src/lib.rs`, `README.md`, configurations) into a clean, ready-to-compile archive.
   - **Open / Resume .ZIP**: Drag-and-drop any previously downloaded `.zip` file into the browser window or click "Open .ZIP" to unpack all files and continue prompt-driven iteration.

4. **Built-In Starter Templates**:
   - **KRONOS Essentials**: Spawns, homes, warps, RTP, and back commands.
   - **KRONOS Ranks**: 20-tier rank hierarchy, tablist formatting, prefixes, and permissions.
   - **KRONOS Guard**: Combat tag, exploit prevention, and attack speed regulation.
   - **KRONOS Regions**: Region claims, wand tool, and protection flags.
   - **Minimal Starter**: Clean `pumpkin-plugin-api` boilerplate with `register_plugin!`.

---

## 🚀 Quick Start (Local Run)

### 1. Install Dependencies
```bash
npm install
```

### 2. Run Development Server
```bash
npm run dev
```
Open [http://localhost:3000](http://localhost:3000) in your browser.

### 3. Set API Key
Click **"Set API Key"** in the top navigation bar and enter your Google AI Studio API key (stored securely in browser `localStorage`). Get a key for free at: [https://aistudio.google.com/app/apikey](https://aistudio.google.com/app/apikey).

---

## ⚡ How to Build & Deploy Generated Plugins

Once you download your plugin's `.zip` file from the app:

1. **Unzip the plugin on your build machine or server**:
   ```bash
   unzip my-plugin.zip -d my-plugin
   cd my-plugin
   ```

2. **Compile to WebAssembly (`wasm32-wasip2`)**:
   ```bash
   cargo build --target wasm32-wasip2 --release
   ```

3. **Deploy to KRONOS Server**:
   Copy the resulting `.wasm` binary into the Pumpkin server's plugins directory:
   ```bash
   cp target/wasm32-wasip2/release/my_plugin.wasm /home/unclebug/pumpkin-server/plugins/
   ```
   Pumpkin will automatically hot-reload or load the plugin on restart!

---

## 📄 License
MIT License. Built for the KRONOS Server Team.
