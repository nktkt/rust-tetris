# Rotation (SRS)

Rotations follow the [Super Rotation System](https://tetris.fandom.com/wiki/SRS) used by the modern Tetris guideline. Each piece has 4 rotation states (`0`, `R`, `2`, `L`), and the rotation handler tries up to 5 offset candidates ("wall kicks") for each transition. The first candidate that doesn't collide is applied.

## Kick tables

Coordinates here use `(dr, dc)` — `dr` is downward and `dc` is rightward, matching the in-game coordinate system. The standard SRS tables are quoted with `y` pointing **up**, so values used in the codebase are pre-converted by `dr = -y`, `dc = x`.

### J, L, S, T, Z

| Transition | Test 1  | Test 2     | Test 3      | Test 4    | Test 5      |
| ---------- | ------- | ---------- | ----------- | --------- | ----------- |
| `0 → R`    | `(0,0)` | `(0,-1)`   | `(-1,-1)`   | `(2,0)`   | `(2,-1)`    |
| `R → 0`    | `(0,0)` | `(0,1)`    | `(1,1)`     | `(-2,0)`  | `(-2,1)`    |
| `R → 2`    | `(0,0)` | `(0,1)`    | `(1,1)`     | `(-2,0)`  | `(-2,1)`    |
| `2 → R`    | `(0,0)` | `(0,-1)`   | `(-1,-1)`   | `(2,0)`   | `(2,-1)`    |
| `2 → L`    | `(0,0)` | `(0,1)`    | `(-1,1)`    | `(2,0)`   | `(2,1)`     |
| `L → 2`    | `(0,0)` | `(0,-1)`   | `(1,-1)`    | `(-2,0)`  | `(-2,-1)`   |
| `L → 0`    | `(0,0)` | `(0,-1)`   | `(1,-1)`    | `(-2,0)`  | `(-2,-1)`   |
| `0 → L`    | `(0,0)` | `(0,1)`    | `(-1,1)`    | `(2,0)`   | `(2,1)`     |

### I piece

The I-piece uses a different table because it pivots between cell edges rather than around a fixed cell. See `src/main.rs::srs_kicks` for the values.

### O piece

The O-piece is rotationally symmetric. Rotation succeeds with no kick.

## Lock delay

After a piece becomes grounded (can't fall further), it has **500 ms** of grace before locking. Any successful move or rotation resets the timer. The reset budget is capped at **15 resets per piece** to prevent infinite stalling. Moving the piece off the ground (e.g., scooting off a ledge) clears the timer entirely.

## DAS / ARR

Auto-shift kicks in only on terminals that support the kitty keyboard protocol (so we can see key releases). On those terminals:

| Parameter             | Value  |
| --------------------- | ------ |
| DAS (delayed auto shift) | 167 ms |
| ARR (auto-repeat rate)   | 33 ms  |
| SDR (soft drop rate)     | 50 ms  |

On terminals without kitty support, you'll get plain OS-rate key repeat instead.
