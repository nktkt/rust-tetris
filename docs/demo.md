# Recording a demo

The static screenshot in [`screenshot.txt`](screenshot.txt) is hand-rendered. To capture a real animated demo, the recommended pipeline is:

## Option A — asciinema (recommended)

```sh
# Record a session
asciinema rec --idle-time-limit 1 --command 'tetris' demo.cast

# Optional: convert to a GIF
agg --theme monokai --speed 1 --rows 28 --cols 60 demo.cast demo.gif

# Or render to SVG (animated)
svg-term --in demo.cast --out demo.svg --window
```

Place the resulting `demo.gif` or `demo.svg` under `docs/` and reference it from the top-level `README.md`.

## Option B — terminal screenshot

A static screenshot also works well on GitHub:

1. Resize your terminal to ~80×30.
2. Run `tetris` and play a few pieces to get a mid-game state.
3. Take an OS screenshot (macOS: <kbd>⌘</kbd>+<kbd>⇧</kbd>+<kbd>4</kbd>, Linux: GNOME Screenshot / `flameshot`).
4. Save as `docs/screenshot.png` and link it from the README.

## Tips

- Use a font with strong box-drawing glyph support (Iosevka, JetBrains Mono, Berkeley Mono).
- A terminal with 256-color/truecolor support gives the correct piece colors.
- `--idle-time-limit 1` keeps recordings tight by trimming long pauses.
