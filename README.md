# Masters Overlay

A lightweight, always-on-top desktop overlay that displays live leaderboard standings for The Masters golf tournament. Built in Rust with [egui](https://github.com/emilk/egui).

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)

## Features

- **Always-on-top** frameless overlay window
- **1/8 screen width**, positioned on the right edge of your display
- **Auto-refreshes** scores every 60 seconds from the Masters API
- **Draggable** title bar (OS-native drag)
- **Resizable height** via bottom drag handle
- **Masters-themed** UI with Augusta green and gold color palette
- **Scrollable** leaderboard with alternating row colors
- **Score coloring** — red for under par, green for over par

## Columns

| POS | PLAYER | PAR | TODAY | THRU |
|-----|--------|-----|-------|------|

## Building

Requires [Rust](https://rustup.rs/) (stable).

```sh
cargo build --release
```

The binary will be at `target/release/masters-overlay.exe`.

## Running

```sh
cargo run --release
```

Or run the binary directly:

```sh
./target/release/masters-overlay.exe
```

## Controls

- **Drag** the title bar to move the window
- **Drag** the bottom edge (grip dots) to resize height
- **Click** the X button to close
- **Scroll** the leaderboard if it overflows

## Data Source

Scores are fetched from `https://www.masters.com/en_US/scores/feeds/2026/scores.json` and refreshed every 60 seconds.

## License

MIT
