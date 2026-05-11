# Rust Tetris

A terminal-based Tetris clone written in Rust, built on top of [`crossterm`](https://crates.io/crates/crossterm).

![demo](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue) ![rust](https://img.shields.io/badge/rust-2021-orange)

## Features

- **Standard 10×20 playfield** with colored Unicode block rendering
- **All seven tetrominoes** (I, O, T, S, Z, J, L) with a **7-bag randomizer** for fair piece distribution
- **Rotation with wall kicks** (simplified SRS-style)
- **Ghost piece** showing the landing position
- **Hold piece** — stash the current piece with `c` and swap it back later
- **Lock delay** — 500 ms grace period after landing, with up to 15 move/rotate resets for fine adjustments
- **Next queue preview** showing the upcoming 5 pieces
- **Scoring** with single/double/triple/Tetris bonuses scaled by level
- **Progressive gravity** — speed increases every 10 lines, down to 80 ms per cell
- **High score persistence** to `~/.tetris_scores` (top 10 kept)
- **Pause / restart / game over** screens

## Install

Requires Rust 1.70+ and Cargo.

```sh
git clone https://github.com/nktkt/rust-tetris.git
cd rust-tetris
cargo install --path .
```

This installs the `tetris` binary into `~/.cargo/bin`.

Alternatively, run directly without installing:

```sh
cargo run --release
```

## Controls

| Key       | Action       |
| --------- | ------------ |
| `←` `→`   | Move left / right |
| `↓`       | Soft drop    |
| `↑` / `x` | Rotate clockwise |
| `z`       | Rotate counter-clockwise |
| `Space`   | Hard drop    |
| `c`       | Hold piece   |
| `p`       | Pause        |
| `r`       | Restart      |
| `q` / `Esc` | Quit       |

## Scoring

| Lines cleared | Base points |
| ------------- | ----------- |
| 1 (Single)    | 100         |
| 2 (Double)    | 300         |
| 3 (Triple)    | 500         |
| 4 (Tetris)    | 800         |

Line clears are multiplied by the current level. Soft drop adds 1 point per cell, hard drop adds 2 points per cell.

The level increases by one for every 10 lines cleared, which also speeds up gravity.

## High Scores

Top 10 scores are saved to `~/.tetris_scores` as plain text (one score per line). The current best is shown in the info panel and updated automatically on game over.

To reset, simply delete the file:

```sh
rm ~/.tetris_scores
```

## Requirements

- A terminal with Unicode and 256-color support
- At least ~30 rows and ~50 columns of terminal space

## Project Layout

```
.
├── Cargo.toml       # dependencies: crossterm, rand
├── src/
│   └── main.rs      # all game logic, rendering, and input handling
└── README.md
```

## License

MIT
