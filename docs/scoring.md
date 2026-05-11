# Scoring

All point values are multiplied by the current **level** (1 + lines / 10).

## Line clears (no spin)

| Clear  | Base points |
| ------ | ----------- |
| Single | 100         |
| Double | 300         |
| Triple | 500         |
| Tetris | 800         |

## T-Spin bonuses

A T-Spin is detected when, on lock, **all** of the following hold:

1. The active piece is a T.
2. The last successful action was a rotation (not a translation).
3. At least 3 of the 4 diagonal cells around the T's center are occupied (or out of bounds).

The T-Spin variant depends on which corners are filled:

- **Full T-Spin**: both *back* corners (on the flat side of the T) are filled.
- **T-Spin Mini**: both *front* corners (next to the T's stem) are filled, but not both back corners.

| Result              | Base points |
| ------------------- | ----------- |
| T-Spin (no lines)   | 400         |
| T-Spin Single       | 800         |
| T-Spin Double       | 1200        |
| T-Spin Triple       | 1600        |
| T-Spin Mini         | 100         |
| T-Spin Mini Single  | 200         |
| T-Spin Mini Double  | 400         |

## Drops

| Action     | Per cell |
| ---------- | -------- |
| Soft drop  | 1        |
| Hard drop  | 2        |

## Level progression

The level increases by one for every **10 lines cleared**. Higher levels accelerate gravity, down to a floor of 80 ms per cell.
