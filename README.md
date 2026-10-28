<p align="center">
  <img src="assets/app-icon/hicolor/256x256/apps/ai.storyteller.gridcraft.png" width="128" height="128" alt="GridCraft icon: a spreadsheet page on a green tile">
</p>

<h1 align="center">GridCraft Mobile</h1>

<p align="center">
  <b>High-performance, pure-Rust spreadsheet application with native Android mobile support.</b>
</p>

<p align="center">
  A mobile-optimized, open-source spreadsheet client powered by the GridCraft engine.<br>
  Runs natively on <b>Android</b>, <b>Windows</b>, <b>macOS</b>, <b>Linux</b>, <b>BSD</b>, and <b>Web (WASM)</b>.
</p>

<p align="center">
  <img alt="Written in Rust" src="https://img.shields.io/badge/written%20in-Rust-147a40?style=flat-square&logo=rust&logoColor=white">
  <img alt="Runs on Android, Desktop and Web" src="https://img.shields.io/badge/platform-Android%20%C2%B7%20Windows%20%C2%B7%20macOS%20%C2%B7%20Linux%20%C2%B7%20Web-1f9d55?style=flat-square">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-147a40?style=flat-square">
  <img alt="Architecture: 17 Crates" src="https://img.shields.io/badge/crates-17%20modules-blue?style=flat-square">
  <br>
  <a href="https://github.com/bagjasatrio/gridcraft-mobile/releases/download/v0.12.1-mobile/GridCraft-Mobile-v0.12.1.apk">
    <img alt="Download Android APK" src="https://img.shields.io/badge/%F0%9F%93%B1%20Download%20Android%20APK-v0.12.1%20Release%20(28%20MB)-04D361?style=for-the-badge&logo=android&logoColor=white">
  </a>
</p>

---

> [!NOTE]
> **About this fork:**
> This repository is an independent, mobile-focused fork of [GridCraft](https://github.com/storytold/gridcraft) (originally developed by the ArtCraft team).
> It introduces native Android packaging, Vulkan/wgpu mobile GPU rendering, dynamic system-bar & Gboard IME insets, responsive multi-orientation layout (portrait/landscape), touch pinch-to-zoom gestures, and direct local XLSX file storage persistence.

---

## 📱 Mobile Features (Android)

1. **Native GPU Acceleration (Vulkan / wgpu)**
   - Immediate-mode UI rendering of spreadsheets, conditional styling, sparklines, and charts directly via Vulkan without WebView overhead.
2. **Dynamic System Insets & Keyboard Handling**
   - Insets queried live via Android JNI (`WindowInsets.Type.systemBars() | WindowInsets.Type.ime()`).
   - Automatically avoids status bar and adjusts viewport when the virtual keyboard (Gboard) is shown.
   - Auto-scrolls the active cell into view when editing.
3. **Adaptive Orientation & Responsive Layout**
   - Free rotation between Landscape and Portrait.
   - Portrait mode displays **41 full rows** on mobile (and 27 rows when Gboard is open).
   - Responsive mobile action bar collapses to compact icons on narrow screens (`⌨`, `−`, `+`, `💾`, `📂 Open`).
4. **Touch Navigation & Zoom**
   - Single-finger tap for cell selection, drag to scroll.
   - Pinch gesture (`zoom_delta`) and quick toolbar buttons (`−` / `+`) hooked directly into engine zoom.
5. **Storage Persistence & Workbook Browser**
   - **Save**: Saves valid `.xlsx` spreadsheets directly into Android application storage (`/sdcard/Android/data/ai.storyteller.gridcraft.spike/files/`).
   - **Open**: In-app modal browser to pick from built-in templates (Sales Dashboard, Budget, Gradebook) or previously saved workbooks.
6. **Optimized Release Packaging**
   - Binary stripped and zip-aligned with keystore signing.
   - APK size optimized from 220 MB (debug) to **28 MB** (universal `aarch64` + `x86_64`).

---

## ⚡ Core Engine Capabilities

- **Familiar Spreadsheet UX:** Ribbon, formula bar, Name Box, sheet tabs, conditional formatting, fill handle, and live charts.
- **XLSX Native Format:** High-fidelity reading and writing for styles, formulas, dynamic arrays, tables, charts, comments, and pivot tables.
- **Dependency Graph Recalculation:** Fast incremental recalc engine supporting 500+ functions, dynamic arrays, `LET`, `LAMBDA`, `XLOOKUP`, `INDEX/MATCH`, etc.
- **Agent & Automation Ready:** Native CLI evaluation and JSON-RPC control protocol support for headless automation and MCP.

---

## 🚀 Quick Start

### Android (Mobile APK)

Direct download:
- **[Download GridCraft-Mobile-v0.12.1.apk](https://github.com/bagjasatrio/gridcraft-mobile/releases/download/v0.12.1-mobile/GridCraft-Mobile-v0.12.1.apk)** (28 MB)

To build from source:
```sh
# Prerequisites: Android SDK + NDK 26+, Rust targets aarch64 & x86_64
cargo install --git https://github.com/parasyte/cargo-apk.git --rev 282639508eeed7d73f2e1eaeea042da2716436d5 cargo-apk

# Build signed release APK
cargo apk build --release -p gridcraft-android-spike --lib

# Install to connected device / emulator
adb install -r target/release/apk/gridcraft-android-spike.apk
adb shell am start -n ai.storyteller.gridcraft.spike/android.app.NativeActivity
```

### Desktop (Windows, macOS, Linux)

```sh
# Run desktop app
cargo run --release -p gridcraft

# Open built-in sample
cargo run --release -p gridcraft -- --sample sales

# Run workspace checks and test suite
cargo check --workspace --all-targets
cargo test --workspace
```

### Web (WebAssembly)

```sh
cd apps/gridcraft-web
trunk serve --release
```

---

## 🏗️ Architecture

GridCraft follows an engine-first, strictly layered Cargo workspace (`cargo xtask layers`):

| Layer | Crates | Description |
|---|---|---|
| **L0** | `core`, `numfmt`, `pdf` | Base types, addresses, values, dates, number format lexer |
| **L1** | `formula`, `functions` | Formula parser, AST, reference math, 500+ worksheet functions |
| **L2** | `model`, `calc`, `xlsx` | Workbook model, dependency graph, evaluator, XLSX/ODS parser |
| **L3** | `chart` | Toolkit-free chart layout and vector drawing |
| **L5** | `engine` | Session manager, commands, transaction history, undo/redo, file I/O |
| **L6** | `ui-egui`, `mcp` | Egui desktop/mobile frontend, canvas renderer, MCP server |
| **L7** | `gridcraft`, `gridcraft-cli`, `gridcraft-web`, `gridcraft-android-spike` | Desktop app, CLI binary, WebAssembly app, Android native app |

---

## 📄 License & Credits

- **Dual Licensed:** GridCraft is distributed under the terms of either the **MIT License** or the **Apache License (Version 2.0)**, at your option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
- **Original Authors:** Original GridCraft codebase copyright (c) 2026 ArtCraft Team and GridCraft contributors ([storytold/gridcraft](https://github.com/storytold/gridcraft)).
- **Mobile Fork:** Maintained and adapted for Android mobile by Muhammad Bagja Satrio ([@bagjasatrio](https://github.com/bagjasatrio)).
- **Third-Party Assets:** Attributions and licenses for fonts and bundled assets are detailed in [ATTRIBUTION.md](ATTRIBUTION.md).
