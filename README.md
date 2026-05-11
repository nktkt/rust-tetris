# Rust Tetris

A terminal-based Tetris clone written in Rust, built on top of [`crossterm`](https://crates.io/crates/crossterm).

[![CI](https://github.com/nktkt/rust-tetris/actions/workflows/ci.yml/badge.svg)](https://github.com/nktkt/rust-tetris/actions/workflows/ci.yml)
![platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
![rust](https://img.shields.io/badge/rust-2021-orange)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

```
=== RUST TETRIS ===
┌────────────────────┐  [Marathon]
│ .  .  .  .  .  .  .  .  .  . │
│ .  .  .  .  .  .  .  .  .  . │  Score: 4200
│ .  .  .  .  .  .  .  .  .  . │  Lines: 14
│ .  .  .  .  .  .  .  .  .  . │  Level: 2
│ .  .  .  .  .  .  .  .  .  . │  Time:  01:24.07
│ .  .  .  .  .  .  .  .  .  . │  Best:  18500
│ .  .  ██ ██ .  .  .  .  .  . │
│ .  .  ██ ██ .  .  .  .  .  . │  HOLD:
│ .  .  ░░ ░░ .  .  .  .  .  . │  ████████
│ .  .  ░░ ░░ .  .  .  .  .  . │
│ .  .  .  .  .  .  .  .  .  . │  NEXT:
│ .  .  .  .  .  .  .  .  .  . │  ██  ██
│ .  .  .  .  .  .  .  .  .  . │  ██████
│ .  .  .  .  .  .  .  .  .  . │
│ .  .  .  .  .  .  .  .  .  . │  ████████
│ .  .  .  .  .  .  .  .  .  . │
│ .  .  .  .  .  .  .  .  .  . │  ██████
│ ██ ██ .  ██ ██ ██ .  ██ ██ ██│    ██
│ ██ .  ██ ██ ██ ██ ██ ██ ██ ██│
└────────────────────┘    ████
                          ████
← → Move │ ↓ Soft │ ↑/x Rotate │ z RotCCW │ SPC Hard │ c Hold │ ...
```

> See [`docs/demo.md`](docs/demo.md) for instructions on recording an animated demo with asciinema or a real screenshot.

## Features

- **Standard 10×20 playfield** with colored Unicode block rendering
- **All seven tetrominoes** (I, O, T, S, Z, J, L) with a **7-bag randomizer** for fair piece distribution
- **Full SRS rotation** with the standard wall-kick tables (and a separate table for the I-piece)
- **T-Spin and T-Spin Mini detection** with guideline bonus scoring
- **Ghost piece** showing the landing position
- **Hold piece** — stash the current piece with `c` and swap it back later
- **Lock delay** — 500 ms grace period after landing, with up to 15 move/rotate resets for fine adjustments
- **DAS / ARR** auto-shift on terminals with kitty keyboard support (167 ms DAS, 33 ms ARR, 50 ms soft drop)
- **Next queue preview** showing the upcoming 5 pieces
- **Three game modes**: Marathon (endless), Sprint (40 lines for time), Ultra (2 minutes for score)
- **Progressive gravity** — speed increases every 10 lines, down to 80 ms per cell
- **Per-mode high scores** persisted under `~/.tetris_scores_{mode}`
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

## Usage

```sh
tetris                       # Marathon (default)
tetris --mode marathon       # endless, increasing gravity
tetris --mode sprint         # clear 40 lines as fast as possible
tetris --mode ultra          # score as much as possible in 2 minutes
```

## Controls

| Key       | Action       |
| --------- | ------------ |
| `←` `→`   | Move left / right (auto-shift on hold) |
| `↓`       | Soft drop (auto-repeat on hold)        |
| `↑` / `x` | Rotate clockwise                       |
| `z`       | Rotate counter-clockwise               |
| `Space`   | Hard drop                              |
| `c`       | Hold piece                             |
| `p`       | Pause                                  |
| `r`       | Restart                                |
| `q` / `Esc` | Quit                                 |

## Scoring (short version)

| Action            | Base points |
| ----------------- | ----------- |
| Single            | 100         |
| Double            | 300         |
| Triple            | 500         |
| Tetris            | 800         |
| T-Spin            | 400         |
| T-Spin Single     | 800         |
| T-Spin Double     | 1200        |
| T-Spin Triple     | 1600        |
| T-Spin Mini       | 100         |
| T-Spin Mini Single | 200        |
| Soft drop / cell  | 1           |
| Hard drop / cell  | 2           |

Line clears are multiplied by the current level. The level rises by one for every 10 lines cleared, which also speeds up gravity.

Full table and T-Spin detection rules: [`docs/scoring.md`](docs/scoring.md).
SRS kick tables and lock delay: [`docs/rotation.md`](docs/rotation.md).

## High Scores

High scores are stored as plain text under `~/.tetris_scores_{mode}` (one score or completion time per line, top 10 kept). The current best is shown in the info panel and updated automatically on game over.

To reset a mode, delete its file:

```sh
rm ~/.tetris_scores_marathon
```

## Requirements

- A terminal with Unicode and 256-color support
- At least ~30 rows and ~60 columns of terminal space
- **DAS / ARR auto-shift** requires a terminal that implements the [kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) (Kitty, WezTerm, Alacritty, Foot, Ghostty, recent iTerm2). On other terminals the game falls back to OS key-repeat.

## Project Layout

```
.
├── .github/workflows/ci.yml  # fmt / clippy / build / test on Linux & macOS
├── Cargo.toml                # dependencies: crossterm, rand
├── docs/
│   ├── demo.md               # how to record a gameplay GIF/SVG
│   ├── rotation.md           # SRS kick tables and lock delay
│   ├── screenshot.txt        # ASCII screenshot used in this README
│   └── scoring.md            # full scoring spec
├── src/
│   └── main.rs               # all game logic, rendering, input, and tests
├── LICENSE
└── README.md
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --release --all-targets -- -D warnings
cargo test --release
```

## License

MIT — see [LICENSE](LICENSE).
