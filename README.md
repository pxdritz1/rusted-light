<div align="center">
  <img src="assets/logo.png" alt="Rusted Light Logo" width="160" />
  <h1>Rusted Light</h1>
  <p><strong>A fast, modular, Spotlight-style application launcher for Linux (X11 & Wayland).</strong></p>
  <p>Built with Rust & GTK4.</p>

  <p>
    <img src="https://img.shields.io/badge/rust-2021%20edition-orange?logo=rust" alt="Rust Edition" />
    <img src="https://img.shields.io/badge/gtk-4.0-blue?logo=gnome" alt="GTK4" />
    <img src="https://img.shields.io/badge/platform-linux-lightgrey?logo=linux" alt="Linux" />
    <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
  </p>
</div>

---

## Overview

**Rusted Light** is a lightweight, responsive keyboard-driven launcher inspired by macOS Spotlight and Raycast. It features an extensible multi-provider search architecture that aggregates results across applications, inline calculations, and filesystem queries with unified fuzzy ranking and zero UI lag.

## Features

- 🚀 **Application Launcher**: Scans system and user XDG `.desktop` files (including Flatpak) with intelligent fuzzy matching powered by [`nucleo-matcher`](https://crates.io/crates/nucleo-matcher) (used in Helix editor).
- 🧮 **Inline Calculator**: Instant mathematical evaluation powered by [`evalexpr`](https://crates.io/crates/evalexpr). Supports operations (`23 * 4`), powers (`2 ^ 10`), trigonometric functions (`sin`, `cos`, `tan`), roots (`sqrt`), logarithms (`log`, `ln`), and constants (`pi`, `e`). Press <kbd>Enter</kbd> to copy the result directly to your clipboard.
- 📁 **Asynchronous File Search**: Fast `$HOME` file finder using the multi-threaded [`ignore`](https://crates.io/crates/ignore) engine (ripgrep) running in a background thread. Automatically prunes heavy directories (`node_modules`, `target`, `.git`, `.cache`, `.cargo`) and respects `.gitignore`. Seamless fallback to `plocate` during startup.
- 🎯 **Unified Ranking**: Providers evaluate queries independently and score matches dynamically without hardcoded prefix rules in the UI layer.
- ⚡ **Non-Blocking UI**: Background indexing and in-memory caches guarantee sub-millisecond query evaluation on every keystroke.

---

## Architecture

Rusted Light decouples UI presentation from search logic via a modular provider system:

```text
src/
├── desktop_entry.rs        # XDG .desktop parsing and app scanner
├── main.rs                 # GTK4 application shell and window
└── providers/
    ├── mod.rs              # SearchProvider trait, SearchResult, SearchAction & ProviderManager
    ├── apps.rs             # Application search provider (nucleo fuzzy matching)
    ├── calc.rs             # Inline calculator provider (evalexpr)
    └── files.rs            # Asynchronous file search provider (in-memory indexer)
```

### The `SearchProvider` Trait

Each mode implements the shared `SearchProvider` trait:

```rust
pub trait SearchProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn priority_boost(&self) -> i64 { 0 }
    fn matches(&self, query: &str) -> Vec<SearchResult>;
}
```

Queries are aggregated and ranked by score using `ProviderManager::search(&str)`.

---

## Getting Started

### Prerequisites

Ensure you have Rust and GTK4 development libraries installed on your Linux system:

#### Arch Linux
```bash
sudo pacman -S rust cargo gtk4
```

#### Ubuntu / Debian
```bash
sudo apt update
sudo apt install cargo libgtk-4-dev build-essential
```

#### Fedora
```bash
sudo dnf install cargo gtk4-devel
```

*(Optional)* For clipboard fallback outside GTK context and instant system search:
```bash
# Wayland
sudo apt install wl-clipboard
# X11
sudo apt install xclip
# Fast locate
sudo apt install plocate
```

### Building & Running

```bash
# Clone the repository
git clone https://github.com/pxdritz1/rusted-light.git
cd rusted-light

# Run in debug mode
cargo run

# Build an optimized release binary
cargo build --release
```

The optimized binary will be available at `target/release/rusted-light`.

---

## Roadmap

- [x] Multi-provider search architecture
- [x] XDG & Flatpak application search with fuzzy ranking
- [x] Inline math calculator with clipboard copy
- [x] Asynchronous `$HOME` file finder with folder actions
- [ ] Unit & currency conversion provider
- [ ] Direct shell command execution mode (`> command`)
- [ ] Web search fallback (`? query`)
- [ ] Configurable hotkeys and Wayland layer-shell protocol integration

---

## License

This project is licensed under the [MIT License](LICENSE).
