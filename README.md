# evtr 🎛️

`evtr` is a Linux-first terminal UI for exploring raw evdev events. Pick an input device, then watch axes, relative movement, and button states stream in with live gauges.

## ✨ Highlights
- 🔍 Fuzzy-search devices from `/dev/input` with live scoring
- 📊 Rich gauges for absolute + relative axes and button grids
- 🧭 Fast navigation: PgUp/PgDn, g/G, Home/End, Esc, Ctrl-U, Ctrl-R

## 🕹️ Flow
- Selector: type to filter, use arrows/PgUp/PgDn/Home/End to move, Enter to confirm, Esc to back out
- Monitor: reset with `r`, jump with `g/G`, scroll with arrows or PgUp/PgDn, Esc to return to the selector, Ctrl-C to exit the app
- Layout adapts to terminal size, keeping axes and mapped buttons responsive

## 🚀 Quick start
- ✅ Ensure your user can read `/dev/input/event*` (sudo or group rule)
- ▶️ `cargo run --release`
- 🧪 Optional: `cargo run -- --test-ui` for scripted snapshots (see `.claude/settings.local.json`)

## ℹ️ Notes
- 🐧 Requires a Linux kernel exposing evdev; run inside a terminal supporting crossterm
- 🔐 Needs permission to open the selected `/dev/input/event*` node
- 🪪 UI is purely local; no data leaves your machine

## 🛠️ Stack
- 🧱 Rust + Tokio + Ratatui + Crossterm
