# Changelog

All notable changes to this project will be documented in this file.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-05-11

### Added

- **Back-to-Back chain** with a ×1.5 multiplier for consecutive difficult clears (Tetris or T-Spin with lines). A non-difficult line clear breaks the chain; a no-clear lock maintains it.
- **Combo counter** with a `50 × combo × level` bonus on every consecutive line clear.
- **Line-clear animation**: cleared rows flash for 120 ms before collapsing. Gameplay input and gravity are paused during the animation.
- Tests for B2B chain, combo progression, and the new full-row / collapse helpers.
- Repo plumbing: `dependabot.yml` (cargo + actions, weekly), `CHANGELOG.md`, Issue and Pull Request templates.
- Published on crates.io as `srs-tetris` (the binary is still named `tetris`).

### Changed

- The clear label shown on the board now includes a `B2B` prefix and `xN COMBO` suffix when applicable.
- `clear_full_lines` was split into `full_rows` + `collapse_rows` to support the animation. The original method is preserved under `#[cfg(test)]` for backwards compatibility with existing tests.
- Crate renamed from `tetris` to `srs-tetris` for crates.io publication (the name `tetris` was already taken).

## [0.1.0] — 2026-05-11

### Added

- Initial public release.
- TUI Tetris in Rust with crossterm rendering.
- Full SRS rotation (with a separate kick table for the I-piece) and T-Spin / T-Spin Mini detection with guideline bonus scoring.
- Hold piece (with single-use cooldown until lock).
- 500 ms lock delay with up to 15 move/rotate resets per piece.
- Ghost piece and 5-piece next preview.
- DAS / ARR / SDR auto-shift on terminals with kitty keyboard protocol support.
- Three game modes: Marathon (endless), Sprint (40 lines), Ultra (2 minutes), selectable via `--mode`.
- Per-mode high-score persistence under `~/.tetris_scores_{mode}`.
- GitHub Actions CI (fmt / clippy / build / test) on Linux and macOS.
- GitHub Actions release pipeline producing Linux, macOS (x86_64 + aarch64), and Windows binaries with SHA256 manifests.

[Unreleased]: https://github.com/nktkt/rust-tetris/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nktkt/rust-tetris/releases/tag/v0.2.0
[0.1.0]: https://github.com/nktkt/rust-tetris/releases/tag/v0.1.0
