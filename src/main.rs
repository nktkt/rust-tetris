use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const WIDTH: usize = 10;
const HEIGHT: usize = 20;
const LOCK_DELAY_MS: u64 = 500;
const MAX_LOCK_RESETS: u8 = 15;
const SCORES_FILE: &str = ".tetris_scores";
const SCORES_KEEP: usize = 10;
const NEXT_PREVIEW: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum Piece {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Piece {
    fn all() -> [Piece; 7] {
        [
            Piece::I,
            Piece::O,
            Piece::T,
            Piece::S,
            Piece::Z,
            Piece::J,
            Piece::L,
        ]
    }

    fn color(self) -> Color {
        match self {
            Piece::I => Color::Cyan,
            Piece::O => Color::Yellow,
            Piece::T => Color::Magenta,
            Piece::S => Color::Green,
            Piece::Z => Color::Red,
            Piece::J => Color::Blue,
            Piece::L => Color::DarkYellow,
        }
    }

    fn shape(self, rot: usize) -> [(i32, i32); 4] {
        let r = rot & 3;
        match self {
            Piece::I => match r {
                0 => [(1, 0), (1, 1), (1, 2), (1, 3)],
                1 => [(0, 2), (1, 2), (2, 2), (3, 2)],
                2 => [(2, 0), (2, 1), (2, 2), (2, 3)],
                _ => [(0, 1), (1, 1), (2, 1), (3, 1)],
            },
            Piece::O => [(0, 1), (0, 2), (1, 1), (1, 2)],
            Piece::T => match r {
                0 => [(0, 1), (1, 0), (1, 1), (1, 2)],
                1 => [(0, 1), (1, 1), (1, 2), (2, 1)],
                2 => [(1, 0), (1, 1), (1, 2), (2, 1)],
                _ => [(0, 1), (1, 0), (1, 1), (2, 1)],
            },
            Piece::S => match r {
                0 => [(0, 1), (0, 2), (1, 0), (1, 1)],
                1 => [(0, 1), (1, 1), (1, 2), (2, 2)],
                2 => [(1, 1), (1, 2), (2, 0), (2, 1)],
                _ => [(0, 0), (1, 0), (1, 1), (2, 1)],
            },
            Piece::Z => match r {
                0 => [(0, 0), (0, 1), (1, 1), (1, 2)],
                1 => [(0, 2), (1, 1), (1, 2), (2, 1)],
                2 => [(1, 0), (1, 1), (2, 1), (2, 2)],
                _ => [(0, 1), (1, 0), (1, 1), (2, 0)],
            },
            Piece::J => match r {
                0 => [(0, 0), (1, 0), (1, 1), (1, 2)],
                1 => [(0, 1), (0, 2), (1, 1), (2, 1)],
                2 => [(1, 0), (1, 1), (1, 2), (2, 2)],
                _ => [(0, 1), (1, 1), (2, 0), (2, 1)],
            },
            Piece::L => match r {
                0 => [(0, 2), (1, 0), (1, 1), (1, 2)],
                1 => [(0, 1), (1, 1), (2, 1), (2, 2)],
                2 => [(1, 0), (1, 1), (1, 2), (2, 0)],
                _ => [(0, 0), (0, 1), (1, 1), (2, 1)],
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TSpinType {
    None,
    Mini,
    Full,
}

// SRS wall-kick offsets in (dr, dc), already converted from canonical (x, y_up).
// The first entry is always the no-kick test.
fn srs_kicks(piece: Piece, from: usize, to: usize) -> [(i32, i32); 5] {
    let f = from & 3;
    let t = to & 3;
    if piece == Piece::O {
        return [(0, 0); 5];
    }
    if piece == Piece::I {
        return match (f, t) {
            (0, 1) => [(0, 0), (0, -2), (0, 1), (1, -2), (-2, 1)],
            (1, 0) => [(0, 0), (0, 2), (0, -1), (-1, 2), (2, -1)],
            (1, 2) => [(0, 0), (0, -1), (0, 2), (-2, -1), (1, 2)],
            (2, 1) => [(0, 0), (0, 1), (0, -2), (2, 1), (-1, -2)],
            (2, 3) => [(0, 0), (0, 2), (0, -1), (-1, 2), (2, -1)],
            (3, 2) => [(0, 0), (0, -2), (0, 1), (1, -2), (-2, 1)],
            (3, 0) => [(0, 0), (0, 1), (0, -2), (2, 1), (-1, -2)],
            (0, 3) => [(0, 0), (0, -1), (0, 2), (-2, -1), (1, 2)],
            _ => [(0, 0); 5],
        };
    }
    // J, L, S, T, Z
    match (f, t) {
        (0, 1) => [(0, 0), (0, -1), (-1, -1), (2, 0), (2, -1)],
        (1, 0) => [(0, 0), (0, 1), (1, 1), (-2, 0), (-2, 1)],
        (1, 2) => [(0, 0), (0, 1), (1, 1), (-2, 0), (-2, 1)],
        (2, 1) => [(0, 0), (0, -1), (-1, -1), (2, 0), (2, -1)],
        (2, 3) => [(0, 0), (0, 1), (-1, 1), (2, 0), (2, 1)],
        (3, 2) => [(0, 0), (0, -1), (1, -1), (-2, 0), (-2, -1)],
        (3, 0) => [(0, 0), (0, -1), (1, -1), (-2, 0), (-2, -1)],
        (0, 3) => [(0, 0), (0, 1), (-1, 1), (2, 0), (2, 1)],
        _ => [(0, 0); 5],
    }
}

struct Active {
    piece: Piece,
    rot: usize,
    row: i32,
    col: i32,
}

impl Active {
    fn cells(&self) -> [(i32, i32); 4] {
        self.cells_with(self.rot, self.row, self.col)
    }

    fn cells_with(&self, rot: usize, row: i32, col: i32) -> [(i32, i32); 4] {
        let mut c = self.piece.shape(rot);
        for p in c.iter_mut() {
            p.0 += row;
            p.1 += col;
        }
        c
    }
}

struct Bag {
    queue: Vec<Piece>,
}

impl Bag {
    fn new() -> Self {
        let mut b = Bag { queue: Vec::new() };
        b.refill();
        b
    }

    fn refill(&mut self) {
        let mut all = Piece::all().to_vec();
        all.shuffle(&mut thread_rng());
        self.queue.extend(all);
    }

    fn ensure(&mut self, n: usize) {
        while self.queue.len() < n {
            self.refill();
        }
    }

    fn next(&mut self) -> Piece {
        self.ensure(1);
        self.queue.remove(0)
    }

    fn peek_n(&mut self, n: usize) -> Vec<Piece> {
        self.ensure(n);
        self.queue.iter().take(n).copied().collect()
    }
}

fn scores_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(SCORES_FILE)
}

fn load_scores() -> Vec<u32> {
    let mut scores: Vec<u32> = fs::read_to_string(scores_path())
        .ok()
        .map(|s| s.lines().filter_map(|l| l.trim().parse().ok()).collect())
        .unwrap_or_default();
    scores.sort_by(|a, b| b.cmp(a));
    scores.truncate(SCORES_KEEP);
    scores
}

fn save_scores(scores: &[u32]) {
    let body = scores
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(scores_path(), body);
}

struct Game {
    board: [[Option<Color>; WIDTH]; HEIGHT],
    active: Active,
    bag: Bag,
    hold: Option<Piece>,
    hold_used: bool,
    lock_timer: Option<Instant>,
    lock_resets: u8,
    score: u32,
    lines: u32,
    level: u32,
    best_score: u32,
    game_over: bool,
    paused: bool,
    score_saved: bool,
    last_was_rotation: bool,
    last_clear_text: Option<String>,
    last_clear_at: Option<Instant>,
}

impl Game {
    fn new(best_score: u32) -> Self {
        let mut bag = Bag::new();
        let piece = bag.next();
        Game {
            board: [[None; WIDTH]; HEIGHT],
            active: Active {
                piece,
                rot: 0,
                row: 0,
                col: 3,
            },
            bag,
            hold: None,
            hold_used: false,
            lock_timer: None,
            lock_resets: 0,
            score: 0,
            lines: 0,
            level: 1,
            best_score,
            game_over: false,
            paused: false,
            score_saved: false,
            last_was_rotation: false,
            last_clear_text: None,
            last_clear_at: None,
        }
    }

    fn collides(&self, cells: &[(i32, i32); 4]) -> bool {
        for &(r, c) in cells {
            if c < 0 || c >= WIDTH as i32 || r >= HEIGHT as i32 {
                return true;
            }
            if r >= 0 && self.board[r as usize][c as usize].is_some() {
                return true;
            }
        }
        false
    }

    fn is_grounded(&self) -> bool {
        let cells = self
            .active
            .cells_with(self.active.rot, self.active.row + 1, self.active.col);
        self.collides(&cells)
    }

    // Update lock-delay state after a successful move/rotate or after a failed gravity tick.
    // `is_action` is true for lateral moves and rotations (they can RESET the timer).
    fn after_move(&mut self, is_action: bool) {
        if self.is_grounded() {
            match self.lock_timer {
                None => {
                    self.lock_timer = Some(Instant::now());
                }
                Some(_) => {
                    if is_action && self.lock_resets < MAX_LOCK_RESETS {
                        self.lock_timer = Some(Instant::now());
                        self.lock_resets += 1;
                    }
                }
            }
        } else {
            self.lock_timer = None;
        }
    }

    fn try_move(&mut self, dr: i32, dc: i32) -> bool {
        let new_cells =
            self.active
                .cells_with(self.active.rot, self.active.row + dr, self.active.col + dc);
        if !self.collides(&new_cells) {
            self.active.row += dr;
            self.active.col += dc;
            self.last_was_rotation = false;
            // Lateral moves (dr == 0) count as resettable actions; downward gravity does not.
            self.after_move(dr == 0);
            true
        } else {
            false
        }
    }

    fn try_rotate(&mut self, dir: i32) -> bool {
        let from = self.active.rot;
        let to = ((from as i32 + dir).rem_euclid(4)) as usize;
        let kicks = srs_kicks(self.active.piece, from, to);
        for k in kicks {
            let cells = self
                .active
                .cells_with(to, self.active.row + k.0, self.active.col + k.1);
            if !self.collides(&cells) {
                self.active.rot = to;
                self.active.row += k.0;
                self.active.col += k.1;
                self.last_was_rotation = true;
                self.after_move(true);
                return true;
            }
        }
        false
    }

    // Standard guideline T-spin detection.
    // Requires the T-piece to be locked immediately after a successful rotation.
    // Counts 4 diagonal corners around the T's center cell.
    // - Full T-spin: both "back" corners filled (and >=3 total).
    // - Mini T-spin: only one back corner filled, but both front corners filled (>=3 total).
    fn detect_tspin(&self) -> TSpinType {
        if !self.last_was_rotation || self.active.piece != Piece::T {
            return TSpinType::None;
        }
        // Corners of 3x3 box around center (1, 1) of the shape.
        let corners = [(0i32, 0i32), (0, 2), (2, 0), (2, 2)];
        let mut filled = [false; 4];
        for (i, (dr, dc)) in corners.iter().enumerate() {
            let r = self.active.row + dr;
            let c = self.active.col + dc;
            let out_of_bounds = c < 0 || c >= WIDTH as i32 || r >= HEIGHT as i32;
            let on_board = r >= 0 && !out_of_bounds && self.board[r as usize][c as usize].is_some();
            filled[i] = out_of_bounds || on_board;
        }
        let count = filled.iter().filter(|&&b| b).count();
        if count < 3 {
            return TSpinType::None;
        }
        // Front (pointy side) and back (flat side) corner indices into `corners`.
        let (front, back): ([usize; 2], [usize; 2]) = match self.active.rot {
            0 => ([0, 1], [2, 3]),
            1 => ([1, 3], [0, 2]),
            2 => ([2, 3], [0, 1]),
            _ => ([0, 2], [1, 3]),
        };
        let back_filled = back.iter().all(|&i| filled[i]);
        let front_filled = front.iter().all(|&i| filled[i]);
        if back_filled {
            TSpinType::Full
        } else if front_filled {
            TSpinType::Mini
        } else {
            TSpinType::None
        }
    }

    fn lock_piece(&mut self) {
        let tspin = self.detect_tspin();
        let color = self.active.piece.color();
        for (r, c) in self.active.cells() {
            if r >= 0 && r < HEIGHT as i32 && c >= 0 && c < WIDTH as i32 {
                self.board[r as usize][c as usize] = Some(color);
            }
        }
        let cleared = self.clear_full_lines();
        self.apply_score(tspin, cleared);
        self.lock_timer = None;
        self.lock_resets = 0;
        self.hold_used = false;
        self.spawn_next();
    }

    fn clear_full_lines(&mut self) -> u32 {
        let mut cleared = 0u32;
        let mut new_board: [[Option<Color>; WIDTH]; HEIGHT] = [[None; WIDTH]; HEIGHT];
        let mut new_row = HEIGHT as i32 - 1;
        for r in (0..HEIGHT).rev() {
            let full = self.board[r].iter().all(|c| c.is_some());
            if full {
                cleared += 1;
            } else if new_row >= 0 {
                new_board[new_row as usize] = self.board[r];
                new_row -= 1;
            }
        }
        self.board = new_board;
        cleared
    }

    fn apply_score(&mut self, tspin: TSpinType, cleared: u32) {
        self.lines += cleared;
        let (points, label): (u32, Option<&'static str>) = match (tspin, cleared) {
            (TSpinType::Full, 0) => (400, Some("T-SPIN")),
            (TSpinType::Full, 1) => (800, Some("T-SPIN SINGLE")),
            (TSpinType::Full, 2) => (1200, Some("T-SPIN DOUBLE")),
            (TSpinType::Full, 3) => (1600, Some("T-SPIN TRIPLE")),
            (TSpinType::Mini, 0) => (100, Some("T-SPIN MINI")),
            (TSpinType::Mini, 1) => (200, Some("T-SPIN MINI SINGLE")),
            (TSpinType::Mini, 2) => (400, Some("T-SPIN MINI DOUBLE")),
            (TSpinType::None, 1) => (100, Some("SINGLE")),
            (TSpinType::None, 2) => (300, Some("DOUBLE")),
            (TSpinType::None, 3) => (500, Some("TRIPLE")),
            (TSpinType::None, 4) => (800, Some("TETRIS")),
            _ => (0, None),
        };
        if points > 0 {
            self.score += points * self.level;
        }
        if let Some(text) = label {
            self.last_clear_text = Some(text.to_string());
            self.last_clear_at = Some(Instant::now());
        }
        if cleared > 0 {
            self.level = 1 + self.lines / 10;
        }
    }

    fn spawn_next(&mut self) {
        let piece = self.bag.next();
        self.active = Active {
            piece,
            rot: 0,
            row: 0,
            col: 3,
        };
        self.last_was_rotation = false;
        if self.collides(&self.active.cells()) {
            self.game_over = true;
        } else {
            self.after_move(false);
        }
    }

    fn hard_drop(&mut self) {
        while self.try_move(1, 0) {
            self.score += 2;
        }
        self.lock_piece();
    }

    fn soft_drop(&mut self) {
        if self.try_move(1, 0) {
            self.score += 1;
        }
    }

    fn gravity_tick(&mut self) {
        if !self.try_move(1, 0) {
            self.after_move(false);
        }
    }

    fn check_lock_delay(&mut self) {
        let should_lock = match self.lock_timer {
            Some(t) => t.elapsed() >= Duration::from_millis(LOCK_DELAY_MS) && self.is_grounded(),
            None => false,
        };
        if should_lock {
            self.lock_piece();
        }
    }

    fn do_hold(&mut self) {
        if self.hold_used {
            return;
        }
        let cur = self.active.piece;
        let new_piece = match self.hold {
            Some(h) => h,
            None => self.bag.next(),
        };
        self.hold = Some(cur);
        self.hold_used = true;
        self.active = Active {
            piece: new_piece,
            rot: 0,
            row: 0,
            col: 3,
        };
        self.lock_timer = None;
        self.lock_resets = 0;
        self.last_was_rotation = false;
        if self.collides(&self.active.cells()) {
            self.game_over = true;
        } else {
            self.after_move(false);
        }
    }

    fn ghost_row(&self) -> i32 {
        let mut r = self.active.row;
        loop {
            let cells = self
                .active
                .cells_with(self.active.rot, r + 1, self.active.col);
            if self.collides(&cells) {
                return r;
            }
            r += 1;
        }
    }

    fn gravity_ms(&self) -> u64 {
        let base: i64 = 800;
        let step: i64 = 60;
        let lvl = self.level as i64 - 1;
        (base - step * lvl).max(80) as u64
    }

    fn on_game_over_save(&mut self) {
        if self.score_saved {
            return;
        }
        let mut scores = load_scores();
        scores.push(self.score);
        scores.sort_by(|a, b| b.cmp(a));
        scores.truncate(SCORES_KEEP);
        save_scores(&scores);
        if let Some(&best) = scores.first() {
            self.best_score = best;
        }
        self.score_saved = true;
    }
}

fn draw_piece_preview<W: Write>(
    out: &mut W,
    x: u16,
    y: u16,
    piece: Option<Piece>,
) -> std::io::Result<()> {
    for r in 0..2u16 {
        queue!(out, cursor::MoveTo(x, y + r), Print("          "))?;
    }
    if let Some(p) = piece {
        let color = p.color();
        // Render rows 0..=1 of the rot=0 shape; the I piece sits on row 1 (visually grounded).
        for (r, c) in p.shape(0) {
            if (0..2).contains(&r) && c >= 0 {
                queue!(
                    out,
                    cursor::MoveTo(x + (c as u16) * 2, y + r as u16),
                    SetForegroundColor(color),
                    Print("██"),
                    ResetColor
                )?;
            }
        }
    }
    Ok(())
}

fn draw<W: Write>(out: &mut W, game: &mut Game) -> std::io::Result<()> {
    queue!(out, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    queue!(
        out,
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::White),
        Print("=== RUST TETRIS ==="),
        ResetColor
    )?;

    let board_x: u16 = 2;
    let board_y: u16 = 2;

    queue!(out, cursor::MoveTo(board_x - 1, board_y - 1), Print("┌"))?;
    for _ in 0..WIDTH * 2 {
        queue!(out, Print("─"))?;
    }
    queue!(out, Print("┐"))?;

    let ghost_r = game.ghost_row();
    let ghost_cells = game
        .active
        .cells_with(game.active.rot, ghost_r, game.active.col);
    let active_cells = game.active.cells();

    for r in 0..HEIGHT {
        queue!(
            out,
            cursor::MoveTo(board_x - 1, board_y + r as u16),
            Print("│")
        )?;
        queue!(out, cursor::MoveTo(board_x, board_y + r as u16))?;
        for c in 0..WIDTH {
            let cell_color: Option<Color> = game.board[r][c];
            let is_active = active_cells
                .iter()
                .any(|&(ar, ac)| ar == r as i32 && ac == c as i32);
            let is_ghost = !is_active
                && ghost_cells
                    .iter()
                    .any(|&(gr, gc)| gr == r as i32 && gc == c as i32);

            if let Some(col) = cell_color {
                queue!(out, SetForegroundColor(col), Print("██"), ResetColor)?;
            } else if is_active {
                queue!(
                    out,
                    SetForegroundColor(game.active.piece.color()),
                    Print("██"),
                    ResetColor
                )?;
            } else if is_ghost {
                queue!(
                    out,
                    SetForegroundColor(game.active.piece.color()),
                    Print("░░"),
                    ResetColor
                )?;
            } else {
                queue!(
                    out,
                    SetForegroundColor(Color::DarkGrey),
                    Print(" ."),
                    ResetColor
                )?;
            }
        }
        queue!(out, Print("│"))?;
    }

    queue!(
        out,
        cursor::MoveTo(board_x - 1, board_y + HEIGHT as u16),
        Print("└")
    )?;
    for _ in 0..WIDTH * 2 {
        queue!(out, Print("─"))?;
    }
    queue!(out, Print("┘"))?;

    // Info panel ────────────────────────────────────────────
    let info_x: u16 = board_x + (WIDTH as u16) * 2 + 3;
    let mut y = board_y;

    queue!(
        out,
        cursor::MoveTo(info_x, y),
        Print(format!("Score: {}", game.score))
    )?;
    y += 1;
    queue!(
        out,
        cursor::MoveTo(info_x, y),
        Print(format!("Lines: {}", game.lines))
    )?;
    y += 1;
    queue!(
        out,
        cursor::MoveTo(info_x, y),
        Print(format!("Level: {}", game.level))
    )?;
    y += 1;
    queue!(
        out,
        cursor::MoveTo(info_x, y),
        SetForegroundColor(Color::Yellow),
        Print(format!("Best:  {}", game.best_score)),
        ResetColor
    )?;
    y += 2;

    queue!(out, cursor::MoveTo(info_x, y), Print("HOLD:"))?;
    y += 1;
    let hold_color = if game.hold_used {
        // Dim by drawing in dark grey when hold is on cooldown
        None
    } else {
        game.hold
    };
    // Always show the held piece; we just convey "used" via the label
    if game.hold_used {
        queue!(
            out,
            cursor::MoveTo(info_x + 6, y - 1),
            SetForegroundColor(Color::DarkGrey),
            Print("(used)"),
            ResetColor
        )?;
    }
    let _ = hold_color;
    draw_piece_preview(out, info_x, y, game.hold)?;
    y += 3;

    queue!(out, cursor::MoveTo(info_x, y), Print("NEXT:"))?;
    y += 1;
    let nexts = game.bag.peek_n(NEXT_PREVIEW);
    for p in nexts {
        draw_piece_preview(out, info_x, y, Some(p))?;
        y += 3;
    }

    // Controls help below board
    let help_y = board_y + HEIGHT as u16 + 2;
    queue!(
        out,
        cursor::MoveTo(0, help_y),
        SetForegroundColor(Color::DarkGrey),
        Print("← → Move │ ↓ Soft │ ↑/x Rotate │ z RotCCW │ SPC Hard │ c Hold │ p Pause │ r Restart │ q Quit"),
        ResetColor
    )?;

    // Flash text for T-spin / Tetris etc. (1500ms fade)
    if let (Some(text), Some(t)) = (game.last_clear_text.as_ref(), game.last_clear_at) {
        if t.elapsed() < Duration::from_millis(1500) {
            let len = text.chars().count() as u16;
            let cx = board_x + (WIDTH as u16) - len.min(WIDTH as u16 * 2) / 2;
            queue!(
                out,
                cursor::MoveTo(cx, board_y + (HEIGHT as u16) / 2 - 2),
                SetForegroundColor(Color::Magenta),
                Print(text),
                ResetColor
            )?;
        }
    }

    if game.paused {
        queue!(
            out,
            cursor::MoveTo(board_x + (WIDTH as u16) - 2, board_y + (HEIGHT as u16) / 2),
            SetForegroundColor(Color::Yellow),
            Print(" PAUSED "),
            ResetColor
        )?;
    }

    if game.game_over {
        queue!(
            out,
            cursor::MoveTo(board_x + (WIDTH as u16) - 5, board_y + (HEIGHT as u16) / 2),
            SetForegroundColor(Color::Red),
            Print(" GAME OVER "),
            ResetColor,
            cursor::MoveTo(
                board_x + (WIDTH as u16) - 9,
                board_y + (HEIGHT as u16) / 2 + 1
            ),
            SetForegroundColor(Color::White),
            Print(" Press r to restart "),
            ResetColor
        )?;
    }

    out.flush()?;
    Ok(())
}

fn run() -> std::io::Result<()> {
    let mut out = stdout();
    terminal::enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, cursor::Hide)?;

    let initial_best = load_scores().first().copied().unwrap_or(0);
    let mut game = Game::new(initial_best);
    let mut last_tick = Instant::now();

    let result = (|| -> std::io::Result<()> {
        loop {
            draw(&mut out, &mut game)?;

            if game.game_over && !game.score_saved {
                game.on_game_over_save();
            }

            let timeout = if game.game_over || game.paused {
                Duration::from_millis(100)
            } else {
                let gravity_remaining =
                    Duration::from_millis(game.gravity_ms()).saturating_sub(last_tick.elapsed());
                let lock_remaining = match game.lock_timer {
                    Some(t) => Duration::from_millis(LOCK_DELAY_MS).saturating_sub(t.elapsed()),
                    None => Duration::from_millis(1000),
                };
                let t = std::cmp::min(gravity_remaining, lock_remaining);
                if t.is_zero() {
                    Duration::from_millis(1)
                } else {
                    t
                }
            };

            if event::poll(timeout)? {
                if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                    if kind == KeyEventKind::Release {
                        continue;
                    }
                    match code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('r') => {
                            let best = game.best_score;
                            game = Game::new(best);
                            last_tick = Instant::now();
                        }
                        KeyCode::Char('p') => {
                            if !game.game_over {
                                game.paused = !game.paused;
                            }
                        }
                        _ => {
                            if game.game_over || game.paused {
                                continue;
                            }
                            match code {
                                KeyCode::Left => {
                                    game.try_move(0, -1);
                                }
                                KeyCode::Right => {
                                    game.try_move(0, 1);
                                }
                                KeyCode::Down => {
                                    game.soft_drop();
                                    last_tick = Instant::now();
                                }
                                KeyCode::Up | KeyCode::Char('x') => {
                                    game.try_rotate(1);
                                }
                                KeyCode::Char('z') => {
                                    game.try_rotate(-1);
                                }
                                KeyCode::Char(' ') => {
                                    game.hard_drop();
                                    last_tick = Instant::now();
                                }
                                KeyCode::Char('c') => {
                                    game.do_hold();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            if !game.game_over && !game.paused {
                if last_tick.elapsed() >= Duration::from_millis(game.gravity_ms()) {
                    game.gravity_tick();
                    last_tick = Instant::now();
                }
                game.check_lock_delay();
            }
        }
        Ok(())
    })();

    execute!(out, cursor::Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn each_piece_shape_has_four_cells() {
        for piece in Piece::all() {
            for rot in 0..4 {
                let cells = piece.shape(rot);
                assert_eq!(cells.len(), 4);
                for (r, c) in cells {
                    assert!(
                        (0..4).contains(&r),
                        "{:?} rot {} row out of 4x4 box",
                        piece,
                        rot
                    );
                    assert!(
                        (0..4).contains(&c),
                        "{:?} rot {} col out of 4x4 box",
                        piece,
                        rot
                    );
                }
            }
        }
    }

    #[test]
    fn bag_first_seven_draws_are_unique() {
        let mut bag = Bag::new();
        let mut seen: HashSet<Piece> = HashSet::new();
        for _ in 0..7 {
            seen.insert(bag.next());
        }
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn bag_peek_does_not_consume() {
        let mut bag = Bag::new();
        let preview = bag.peek_n(5);
        assert_eq!(preview.len(), 5);
        for expected in preview {
            assert_eq!(bag.next(), expected);
        }
    }

    #[test]
    fn collide_detects_walls_and_floor() {
        let game = Game::new(0);
        assert!(game.collides(&[(0, -1), (0, 0), (0, 0), (0, 0)]));
        assert!(game.collides(&[(0, WIDTH as i32), (0, 0), (0, 0), (0, 0)]));
        assert!(game.collides(&[(HEIGHT as i32, 0), (0, 0), (0, 0), (0, 0)]));
        // Cells above the visible field (negative row) should not collide
        assert!(!game.collides(&[(-1, 0), (-1, 1), (0, 0), (0, 1)]));
    }

    #[test]
    fn collide_detects_occupied_cells() {
        let mut game = Game::new(0);
        game.board[5][3] = Some(Color::Red);
        assert!(game.collides(&[(5, 3), (0, 0), (0, 1), (0, 2)]));
        assert!(!game.collides(&[(0, 0), (0, 1), (0, 2), (0, 3)]));
    }

    #[test]
    fn clear_full_lines_clears_one_and_shifts() {
        let mut game = Game::new(0);
        for c in 0..WIDTH {
            game.board[HEIGHT - 1][c] = Some(Color::Red);
        }
        for c in 0..5 {
            game.board[HEIGHT - 2][c] = Some(Color::Blue);
        }
        let cleared = game.clear_full_lines();
        assert_eq!(cleared, 1);
        for c in 0..5 {
            assert_eq!(game.board[HEIGHT - 1][c], Some(Color::Blue));
        }
        for c in 5..WIDTH {
            assert!(game.board[HEIGHT - 1][c].is_none());
        }
    }

    #[test]
    fn clear_full_lines_clears_tetris() {
        let mut game = Game::new(0);
        for r in (HEIGHT - 4)..HEIGHT {
            for c in 0..WIDTH {
                game.board[r][c] = Some(Color::Red);
            }
        }
        let cleared = game.clear_full_lines();
        assert_eq!(cleared, 4);
        for r in 0..HEIGHT {
            for c in 0..WIDTH {
                assert!(game.board[r][c].is_none());
            }
        }
    }

    #[test]
    fn scoring_basic_clears_at_level_one() {
        let mut game = Game::new(0);
        game.level = 1;
        game.apply_score(TSpinType::None, 1);
        assert_eq!(game.score, 100);
        game.apply_score(TSpinType::None, 4);
        assert_eq!(game.score, 100 + 800);
    }

    #[test]
    fn scoring_tspin_bonuses() {
        let mut game = Game::new(0);
        game.level = 1;
        game.apply_score(TSpinType::Full, 0);
        assert_eq!(game.score, 400);
        game.apply_score(TSpinType::Full, 2);
        assert_eq!(game.score, 400 + 1200);
        game.apply_score(TSpinType::Mini, 1);
        assert_eq!(game.score, 1600 + 200);
    }

    #[test]
    fn scoring_scales_with_level() {
        let mut game = Game::new(0);
        game.level = 5;
        game.apply_score(TSpinType::None, 1);
        assert_eq!(game.score, 500);
    }

    #[test]
    fn level_advances_every_ten_lines() {
        let mut game = Game::new(0);
        for _ in 0..10 {
            game.apply_score(TSpinType::None, 1);
        }
        assert_eq!(game.lines, 10);
        assert_eq!(game.level, 2);
    }

    #[test]
    fn ghost_row_finds_floor_for_o_piece() {
        let mut game = Game::new(0);
        game.active = Active {
            piece: Piece::O,
            rot: 0,
            row: 0,
            col: 4,
        };
        assert_eq!(game.ghost_row(), HEIGHT as i32 - 2);
    }

    #[test]
    fn srs_kicks_o_returns_only_zero_offset() {
        for from in 0..4 {
            for to in 0..4 {
                for k in srs_kicks(Piece::O, from, to) {
                    assert_eq!(k, (0, 0));
                }
            }
        }
    }

    #[test]
    fn srs_kicks_first_test_is_always_zero() {
        for piece in Piece::all() {
            for from in 0..4 {
                for to in 0..4 {
                    if from == to {
                        continue;
                    }
                    let kicks = srs_kicks(piece, from, to);
                    assert_eq!(kicks[0], (0, 0));
                }
            }
        }
    }

    #[test]
    fn rotation_succeeds_on_empty_board() {
        let mut game = Game::new(0);
        assert!(game.try_rotate(1));
        assert_eq!(game.active.rot, 1);
        assert!(game.last_was_rotation);
        // Lateral move resets the rotation flag
        game.try_move(0, 1);
        assert!(!game.last_was_rotation);
    }

    #[test]
    fn detect_tspin_requires_t_piece() {
        let mut game = Game::new(0);
        game.active = Active {
            piece: Piece::I,
            rot: 0,
            row: 0,
            col: 3,
        };
        game.last_was_rotation = true;
        assert_eq!(game.detect_tspin(), TSpinType::None);
    }

    #[test]
    fn detect_tspin_requires_recent_rotation() {
        let mut game = Game::new(0);
        game.active = Active {
            piece: Piece::T,
            rot: 0,
            row: HEIGHT as i32 - 3,
            col: 0,
        };
        game.last_was_rotation = false;
        game.board[HEIGHT - 1][0] = Some(Color::Red);
        game.board[HEIGHT - 1][2] = Some(Color::Red);
        game.board[HEIGHT - 3][0] = Some(Color::Red);
        assert_eq!(game.detect_tspin(), TSpinType::None);
    }

    #[test]
    fn detect_tspin_full_when_both_back_corners_filled() {
        let mut game = Game::new(0);
        game.active = Active {
            piece: Piece::T,
            rot: 0,
            row: HEIGHT as i32 - 3,
            col: 0,
        };
        game.last_was_rotation = true;
        // Back corners (rot 0): (2,0) and (2,2) relative = (HEIGHT-1, 0), (HEIGHT-1, 2)
        game.board[HEIGHT - 1][0] = Some(Color::Red);
        game.board[HEIGHT - 1][2] = Some(Color::Red);
        // One front corner to satisfy count >= 3
        game.board[HEIGHT - 3][0] = Some(Color::Red);
        assert_eq!(game.detect_tspin(), TSpinType::Full);
    }

    #[test]
    fn detect_tspin_mini_when_only_front_corners_filled() {
        let mut game = Game::new(0);
        game.active = Active {
            piece: Piece::T,
            rot: 0,
            row: HEIGHT as i32 - 3,
            col: 0,
        };
        game.last_was_rotation = true;
        // Front corners (rot 0): (0,0) and (0,2) relative
        game.board[HEIGHT - 3][0] = Some(Color::Red);
        game.board[HEIGHT - 3][2] = Some(Color::Red);
        // Only one back corner
        game.board[HEIGHT - 1][0] = Some(Color::Red);
        assert_eq!(game.detect_tspin(), TSpinType::Mini);
    }

    #[test]
    fn after_move_starts_lock_timer_when_grounded() {
        let mut game = Game::new(0);
        // Fill the row directly below where the piece will land
        // Use an O piece for simplicity
        game.active = Active {
            piece: Piece::O,
            rot: 0,
            row: HEIGHT as i32 - 2,
            col: 4,
        };
        game.lock_timer = None;
        game.after_move(false);
        assert!(
            game.lock_timer.is_some(),
            "should set lock timer when grounded"
        );
    }

    #[test]
    fn after_move_clears_lock_timer_when_airborne() {
        let mut game = Game::new(0);
        game.active = Active {
            piece: Piece::O,
            rot: 0,
            row: 0,
            col: 4,
        };
        game.lock_timer = Some(Instant::now());
        game.after_move(true);
        assert!(game.lock_timer.is_none());
    }
}
