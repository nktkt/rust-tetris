use crossterm::{
    cursor,
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
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
const SCORES_KEEP: usize = 10;
const NEXT_PREVIEW: usize = 5;
const SPRINT_LINES: u32 = 40;
const ULTRA_SECS: u64 = 120;
const DAS_MS: u64 = 167;
const ARR_MS: u64 = 33;
const SDR_MS: u64 = 50;
const CLEAR_ANIM_MS: u64 = 120;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Marathon,
    Sprint,
    Ultra,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Marathon => "Marathon",
            Mode::Sprint => "Sprint",
            Mode::Ultra => "Ultra",
        }
    }

    fn scores_filename(self) -> &'static str {
        match self {
            Mode::Marathon => ".tetris_scores_marathon",
            Mode::Sprint => ".tetris_scores_sprint",
            Mode::Ultra => ".tetris_scores_ultra",
        }
    }

    fn from_arg(s: &str) -> Option<Mode> {
        match s.to_ascii_lowercase().as_str() {
            "marathon" => Some(Mode::Marathon),
            "sprint" => Some(Mode::Sprint),
            "ultra" => Some(Mode::Ultra),
            _ => None,
        }
    }
}

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

fn scores_path(mode: Mode) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(mode.scores_filename())
}

// Scores are stored highest-first for Marathon/Ultra; for Sprint we store the
// 40-line completion times (milliseconds), lowest-first.
fn load_scores(mode: Mode) -> Vec<u32> {
    let mut scores: Vec<u32> = fs::read_to_string(scores_path(mode))
        .ok()
        .map(|s| s.lines().filter_map(|l| l.trim().parse().ok()).collect())
        .unwrap_or_default();
    sort_scores(mode, &mut scores);
    scores.truncate(SCORES_KEEP);
    scores
}

fn save_scores(mode: Mode, scores: &[u32]) {
    let body = scores
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(scores_path(mode), body);
}

fn sort_scores(mode: Mode, scores: &mut [u32]) {
    match mode {
        // Lower time is better
        Mode::Sprint => scores.sort(),
        // Higher score is better
        Mode::Marathon | Mode::Ultra => scores.sort_by(|a, b| b.cmp(a)),
    }
}

fn format_duration_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    let centi = (ms % 1000) / 10;
    format!("{:02}:{:02}.{:02}", minutes, seconds, centi)
}

struct Game {
    mode: Mode,
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
    started_at: Instant,
    final_time_ms: Option<u64>,
    game_over: bool,
    paused: bool,
    score_saved: bool,
    last_was_rotation: bool,
    last_clear_text: Option<String>,
    last_clear_at: Option<Instant>,
    // Back-to-Back chain is "active" once a difficult clear (Tetris or
    // T-Spin with lines) is performed, and stays active across no-clear
    // locks. A non-difficult line clear breaks it.
    b2b_active: bool,
    // Combo counter: -1 = no current combo, 0+ = number of consecutive
    // clears in this combo minus one (so 0 means "second clear in a row").
    combo: i32,
    // When Some, the listed rows are being flashed before being collapsed.
    // While present, gameplay (gravity / input movement) is frozen.
    clearing_rows: Option<(Vec<usize>, Instant)>,
}

impl Game {
    fn new(mode: Mode, best_score: u32) -> Self {
        let mut bag = Bag::new();
        let piece = bag.next();
        Game {
            mode,
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
            started_at: Instant::now(),
            final_time_ms: None,
            game_over: false,
            paused: false,
            score_saved: false,
            last_was_rotation: false,
            last_clear_text: None,
            last_clear_at: None,
            b2b_active: false,
            combo: -1,
            clearing_rows: None,
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.final_time_ms
            .unwrap_or_else(|| self.started_at.elapsed().as_millis() as u64)
    }

    fn time_remaining_ms(&self) -> u64 {
        let limit = ULTRA_SECS * 1000;
        let elapsed = self.elapsed_ms();
        limit.saturating_sub(elapsed)
    }

    fn lines_remaining(&self) -> u32 {
        SPRINT_LINES.saturating_sub(self.lines)
    }

    // Returns true if the mode's terminating condition has been hit (and freezes the final time).
    fn check_mode_end(&mut self) -> bool {
        if self.game_over {
            return true;
        }
        match self.mode {
            Mode::Marathon => false,
            Mode::Sprint => {
                if self.lines >= SPRINT_LINES {
                    self.final_time_ms = Some(self.elapsed_ms());
                    self.game_over = true;
                    true
                } else {
                    false
                }
            }
            Mode::Ultra => {
                if self.elapsed_ms() >= ULTRA_SECS * 1000 {
                    self.final_time_ms = Some(ULTRA_SECS * 1000);
                    self.game_over = true;
                    true
                } else {
                    false
                }
            }
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
        let full_rows = self.full_rows();
        let cleared = full_rows.len() as u32;
        self.apply_score(tspin, cleared);
        if cleared > 0 {
            // Defer the actual collapse so the rows flash for CLEAR_ANIM_MS.
            self.clearing_rows = Some((full_rows, Instant::now()));
            self.lock_timer = None;
            self.lock_resets = 0;
        } else {
            self.lock_timer = None;
            self.lock_resets = 0;
            self.hold_used = false;
            self.spawn_next();
        }
    }

    fn full_rows(&self) -> Vec<usize> {
        (0..HEIGHT)
            .filter(|&r| self.board[r].iter().all(|c| c.is_some()))
            .collect()
    }

    fn collapse_rows(&mut self, rows: &[usize]) {
        let mut new_board: [[Option<Color>; WIDTH]; HEIGHT] = [[None; WIDTH]; HEIGHT];
        let mut new_row = HEIGHT as i32 - 1;
        for r in (0..HEIGHT).rev() {
            if !rows.contains(&r) && new_row >= 0 {
                new_board[new_row as usize] = self.board[r];
                new_row -= 1;
            }
        }
        self.board = new_board;
    }

    // Kept for tests / backwards compat: synchronously detect + remove full rows
    // without any animation. Returns the number of rows cleared.
    #[cfg(test)]
    fn clear_full_lines(&mut self) -> u32 {
        let rows = self.full_rows();
        let n = rows.len() as u32;
        self.collapse_rows(&rows);
        n
    }

    // Steps a pending clear-animation. Returns true if the collapse just fired.
    fn step_clear_animation(&mut self) -> bool {
        let due = match &self.clearing_rows {
            Some((_, t)) => t.elapsed() >= Duration::from_millis(CLEAR_ANIM_MS),
            None => false,
        };
        if !due {
            return false;
        }
        if let Some((rows, _)) = self.clearing_rows.take() {
            self.collapse_rows(&rows);
            self.hold_used = false;
            self.spawn_next();
            true
        } else {
            false
        }
    }

    fn is_animating(&self) -> bool {
        self.clearing_rows.is_some()
    }

    fn apply_score(&mut self, tspin: TSpinType, cleared: u32) {
        self.lines += cleared;
        let (base, label): (u32, Option<&'static str>) = match (tspin, cleared) {
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

        // A clear is "difficult" if it's a Tetris or any T-Spin with lines.
        let is_difficult = cleared > 0 && (tspin != TSpinType::None || cleared == 4);
        let b2b_continued = is_difficult && self.b2b_active;
        let combo_count = if cleared > 0 { self.combo + 1 } else { -1 };

        // Base + B2B multiplier
        let mut points = if b2b_continued { base * 3 / 2 } else { base };
        // Combo bonus
        if combo_count > 0 {
            points += 50 * combo_count as u32;
        }
        if points > 0 {
            self.score += points * self.level;
        }

        // Build display label
        let mut parts: Vec<String> = Vec::new();
        if b2b_continued {
            parts.push("B2B".into());
        }
        if let Some(s) = label {
            parts.push(s.into());
        }
        if combo_count > 0 {
            parts.push(format!("x{} COMBO", combo_count + 1));
        }
        if !parts.is_empty() {
            self.last_clear_text = Some(parts.join(" "));
            self.last_clear_at = Some(Instant::now());
        }

        // Update B2B / combo state for next lock
        self.combo = combo_count;
        if cleared > 0 {
            self.b2b_active = is_difficult;
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
        // What we save depends on the mode.
        // Sprint: the elapsed time (ms) when 40 lines were cleared — only saved on completion.
        // Marathon / Ultra: the final score.
        let metric: Option<u32> = match self.mode {
            Mode::Sprint => {
                if self.lines >= SPRINT_LINES {
                    Some(self.elapsed_ms().min(u32::MAX as u64) as u32)
                } else {
                    None
                }
            }
            Mode::Marathon | Mode::Ultra => Some(self.score),
        };
        let mut scores = load_scores(self.mode);
        if let Some(m) = metric {
            scores.push(m);
            sort_scores(self.mode, &mut scores);
            scores.truncate(SCORES_KEEP);
            save_scores(self.mode, &scores);
        }
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

    // While a clear animation is playing, alternate the cleared rows
    // between bright white and blank to make them flash.
    let clearing_row = |r: usize| -> Option<bool> {
        let (rows, t) = game.clearing_rows.as_ref()?;
        if !rows.contains(&r) {
            return None;
        }
        // ~30ms strobe
        let phase = (t.elapsed().as_millis() / 30) % 2 == 0;
        Some(phase)
    };

    for r in 0..HEIGHT {
        queue!(
            out,
            cursor::MoveTo(board_x - 1, board_y + r as u16),
            Print("│")
        )?;
        queue!(out, cursor::MoveTo(board_x, board_y + r as u16))?;
        let flash = clearing_row(r);
        for c in 0..WIDTH {
            let cell_color: Option<Color> = game.board[r][c];
            let is_active = active_cells
                .iter()
                .any(|&(ar, ac)| ar == r as i32 && ac == c as i32);
            let is_ghost = !is_active
                && ghost_cells
                    .iter()
                    .any(|&(gr, gc)| gr == r as i32 && gc == c as i32);

            if let Some(phase) = flash {
                if phase {
                    queue!(
                        out,
                        SetForegroundColor(Color::White),
                        Print("██"),
                        ResetColor
                    )?;
                } else {
                    queue!(out, Print("  "))?;
                }
            } else if let Some(col) = cell_color {
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
        SetForegroundColor(Color::Cyan),
        Print(format!("[{}]", game.mode.name())),
        ResetColor
    )?;
    y += 2;
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
    // Mode-specific row
    let mode_line = match game.mode {
        Mode::Marathon => format!("Time:  {}", format_duration_ms(game.elapsed_ms())),
        Mode::Sprint => format!(
            "Left:  {}  ({})",
            game.lines_remaining(),
            format_duration_ms(game.elapsed_ms())
        ),
        Mode::Ultra => format!("Time:  {}", format_duration_ms(game.time_remaining_ms())),
    };
    queue!(out, cursor::MoveTo(info_x, y), Print(mode_line))?;
    y += 1;
    let best_label = if game.mode == Mode::Sprint && game.best_score > 0 {
        format!("Best:  {}", format_duration_ms(game.best_score as u64))
    } else {
        format!("Best:  {}", game.best_score)
    };
    queue!(
        out,
        cursor::MoveTo(info_x, y),
        SetForegroundColor(Color::Yellow),
        Print(best_label),
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

fn parse_mode_arg() -> Result<Mode, String> {
    let mut args = std::env::args().skip(1);
    let mut mode = Mode::Marathon;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--mode" => {
                let v = args
                    .next()
                    .ok_or_else(|| "--mode requires a value".to_string())?;
                mode = Mode::from_arg(&v).ok_or_else(|| {
                    format!("unknown mode: {} (expected marathon|sprint|ultra)", v)
                })?;
            }
            "-h" | "--help" => {
                println!("Usage: tetris [--mode marathon|sprint|ultra]");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {}", a)),
        }
    }
    Ok(mode)
}

#[derive(Default)]
struct DirHeld {
    held_since: Option<Instant>,
    das_satisfied: bool,
    last_shift_at: Option<Instant>,
}

impl DirHeld {
    fn press(&mut self) {
        self.held_since = Some(Instant::now());
        self.das_satisfied = false;
        self.last_shift_at = None;
    }
    fn release(&mut self) {
        self.held_since = None;
        self.das_satisfied = false;
        self.last_shift_at = None;
    }
    // Returns how many shifts to apply this frame.
    fn tick(&mut self, das: Duration, rate: Duration) -> usize {
        let Some(since) = self.held_since else {
            return 0;
        };
        let now = Instant::now();
        if !self.das_satisfied {
            if now.duration_since(since) >= das {
                self.das_satisfied = true;
                self.last_shift_at = Some(now);
                return 1;
            }
            return 0;
        }
        let rate = rate.max(Duration::from_millis(1));
        let mut last = self.last_shift_at.unwrap_or(now);
        let mut count = 0;
        while now.saturating_duration_since(last) >= rate {
            count += 1;
            last += rate;
            if count >= 30 {
                break;
            }
        }
        if count > 0 {
            self.last_shift_at = Some(last);
        }
        count
    }
}

#[derive(Default)]
struct Input {
    // Only enabled once we observe a key Release event — proves the terminal
    // supports the kitty keyboard protocol. Otherwise we fall back to bare
    // OS key-repeat semantics for everyone.
    arr_enabled: bool,
    left: DirHeld,
    right: DirHeld,
    down: DirHeld,
}

impl Input {
    fn on_press(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left => {
                self.left.press();
                self.right.release();
            }
            KeyCode::Right => {
                self.right.press();
                self.left.release();
            }
            KeyCode::Down => self.down.press(),
            _ => {}
        }
    }
    fn on_release(&mut self, code: KeyCode) {
        self.arr_enabled = true;
        match code {
            KeyCode::Left => self.left.release(),
            KeyCode::Right => self.right.release(),
            KeyCode::Down => self.down.release(),
            _ => {}
        }
    }
}

fn run(mode: Mode) -> std::io::Result<()> {
    let mut out = stdout();
    terminal::enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, cursor::Hide)?;
    // Request key release/repeat events. Silently no-op on terminals that
    // don't implement the kitty keyboard protocol.
    let pushed_kbd_flags = execute!(
        out,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    )
    .is_ok();

    let initial_best = load_scores(mode).first().copied().unwrap_or(0);
    let mut game = Game::new(mode, initial_best);
    let mut last_tick = Instant::now();
    let mut input = Input::default();

    let result = (|| -> std::io::Result<()> {
        loop {
            draw(&mut out, &mut game)?;

            if !game.game_over {
                game.check_mode_end();
            }
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
                let mode_remaining = if game.mode == Mode::Ultra {
                    Duration::from_millis(100)
                } else {
                    Duration::from_secs(1)
                };
                // When auto-shift is active, wake up at least every ARR/SDR
                // interval so we can fire repeats smoothly.
                let arr_remaining = if input.arr_enabled
                    && (input.left.held_since.is_some()
                        || input.right.held_since.is_some()
                        || input.down.held_since.is_some())
                {
                    Duration::from_millis(ARR_MS.min(SDR_MS))
                } else {
                    Duration::from_secs(1)
                };
                let anim_remaining = match &game.clearing_rows {
                    Some((_, t)) => {
                        Duration::from_millis(CLEAR_ANIM_MS).saturating_sub(t.elapsed())
                    }
                    None => Duration::from_secs(1),
                };
                let t = gravity_remaining
                    .min(lock_remaining)
                    .min(mode_remaining)
                    .min(arr_remaining)
                    .min(anim_remaining);
                if t.is_zero() {
                    Duration::from_millis(1)
                } else {
                    t
                }
            };

            if event::poll(timeout)? {
                if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                    match kind {
                        KeyEventKind::Release => {
                            input.on_release(code);
                            continue;
                        }
                        KeyEventKind::Repeat => {
                            input.arr_enabled = true;
                            // Don't re-apply: held-key auto-shift handles it.
                            continue;
                        }
                        KeyEventKind::Press => {}
                    }
                    match code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('r') => {
                            let best = game.best_score;
                            game = Game::new(mode, best);
                            last_tick = Instant::now();
                            input = Input {
                                arr_enabled: input.arr_enabled,
                                ..Default::default()
                            };
                        }
                        KeyCode::Char('p') => {
                            if !game.game_over {
                                game.paused = !game.paused;
                            }
                        }
                        _ => {
                            if game.game_over || game.paused || game.is_animating() {
                                continue;
                            }
                            input.on_press(code);
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
                // Always advance the clear animation; this can spawn the next piece.
                if game.step_clear_animation() {
                    last_tick = Instant::now();
                }
                if !game.is_animating() {
                    if input.arr_enabled {
                        let das = Duration::from_millis(DAS_MS);
                        let arr = Duration::from_millis(ARR_MS);
                        let sdr = Duration::from_millis(SDR_MS);
                        let left_n = input.left.tick(das, arr);
                        for _ in 0..left_n {
                            if !game.try_move(0, -1) {
                                break;
                            }
                        }
                        let right_n = input.right.tick(das, arr);
                        for _ in 0..right_n {
                            if !game.try_move(0, 1) {
                                break;
                            }
                        }
                        let down_n = input.down.tick(sdr, sdr);
                        if down_n > 0 {
                            for _ in 0..down_n {
                                game.soft_drop();
                            }
                            last_tick = Instant::now();
                        }
                    }
                    if last_tick.elapsed() >= Duration::from_millis(game.gravity_ms()) {
                        game.gravity_tick();
                        last_tick = Instant::now();
                    }
                    game.check_lock_delay();
                }
            }
        }
        Ok(())
    })();

    if pushed_kbd_flags {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    execute!(out, cursor::Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

fn main() {
    let mode = match parse_mode_arg() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            eprintln!("usage: tetris [--mode marathon|sprint|ultra]");
            std::process::exit(2);
        }
    };
    if let Err(e) = run(mode) {
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
        let game = Game::new(Mode::Marathon, 0);
        assert!(game.collides(&[(0, -1), (0, 0), (0, 0), (0, 0)]));
        assert!(game.collides(&[(0, WIDTH as i32), (0, 0), (0, 0), (0, 0)]));
        assert!(game.collides(&[(HEIGHT as i32, 0), (0, 0), (0, 0), (0, 0)]));
        // Cells above the visible field (negative row) should not collide
        assert!(!game.collides(&[(-1, 0), (-1, 1), (0, 0), (0, 1)]));
    }

    #[test]
    fn collide_detects_occupied_cells() {
        let mut game = Game::new(Mode::Marathon, 0);
        game.board[5][3] = Some(Color::Red);
        assert!(game.collides(&[(5, 3), (0, 0), (0, 1), (0, 2)]));
        assert!(!game.collides(&[(0, 0), (0, 1), (0, 2), (0, 3)]));
    }

    #[test]
    fn clear_full_lines_clears_one_and_shifts() {
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
        game.level = 1;
        game.apply_score(TSpinType::None, 1);
        // single, no combo bonus yet
        assert_eq!(game.score, 100);
        game.apply_score(TSpinType::None, 4);
        // tetris (800) + combo x2 bonus (50)
        assert_eq!(game.score, 100 + 800 + 50);
    }

    #[test]
    fn scoring_tspin_bonuses() {
        let mut game = Game::new(Mode::Marathon, 0);
        game.level = 1;
        // No-line T-Spin: 400 base, no combo (cleared==0), no b2b update
        game.apply_score(TSpinType::Full, 0);
        assert_eq!(game.score, 400);
        // T-Spin Double: 1200 base, first difficult clear with lines → no B2B multiplier
        // yet (b2b_active was false). combo becomes 0, no bonus.
        game.apply_score(TSpinType::Full, 2);
        assert_eq!(game.score, 400 + 1200);
        // T-Spin Mini Single: B2B chains (×1.5 → 300) + combo x2 (+50)
        game.apply_score(TSpinType::Mini, 1);
        assert_eq!(game.score, 1600 + 300 + 50);
    }

    #[test]
    fn b2b_tetris_chain_applies_multiplier() {
        let mut game = Game::new(Mode::Marathon, 0);
        game.level = 1;
        game.apply_score(TSpinType::None, 4);
        assert_eq!(game.score, 800);
        assert!(game.b2b_active);
        // Second tetris: 800 × 1.5 = 1200 base + combo x2 (+50)
        game.apply_score(TSpinType::None, 4);
        assert_eq!(game.score, 800 + 1200 + 50);
    }

    #[test]
    fn b2b_broken_by_non_difficult_clear() {
        let mut game = Game::new(Mode::Marathon, 0);
        game.apply_score(TSpinType::None, 4);
        assert!(game.b2b_active);
        game.apply_score(TSpinType::None, 1);
        assert!(!game.b2b_active);
    }

    #[test]
    fn b2b_maintained_through_no_clear() {
        let mut game = Game::new(Mode::Marathon, 0);
        game.apply_score(TSpinType::None, 4);
        assert!(game.b2b_active);
        game.apply_score(TSpinType::None, 0);
        assert!(game.b2b_active, "no-clear lock must NOT break B2B");
        assert_eq!(game.combo, -1, "no-clear lock resets combo");
    }

    #[test]
    fn combo_advances_and_resets() {
        let mut game = Game::new(Mode::Marathon, 0);
        game.apply_score(TSpinType::None, 1);
        assert_eq!(game.combo, 0);
        game.apply_score(TSpinType::None, 2);
        assert_eq!(game.combo, 1);
        game.apply_score(TSpinType::None, 0);
        assert_eq!(game.combo, -1);
    }

    #[test]
    fn full_rows_and_collapse_rows() {
        let mut game = Game::new(Mode::Marathon, 0);
        for c in 0..WIDTH {
            game.board[HEIGHT - 1][c] = Some(Color::Red);
        }
        let rows = game.full_rows();
        assert_eq!(rows, vec![HEIGHT - 1]);
        game.collapse_rows(&rows);
        assert!(game.board[HEIGHT - 1].iter().all(|c| c.is_none()));
    }

    #[test]
    fn scoring_scales_with_level() {
        let mut game = Game::new(Mode::Marathon, 0);
        game.level = 5;
        game.apply_score(TSpinType::None, 1);
        assert_eq!(game.score, 500);
    }

    #[test]
    fn level_advances_every_ten_lines() {
        let mut game = Game::new(Mode::Marathon, 0);
        for _ in 0..10 {
            game.apply_score(TSpinType::None, 1);
        }
        assert_eq!(game.lines, 10);
        assert_eq!(game.level, 2);
    }

    #[test]
    fn ghost_row_finds_floor_for_o_piece() {
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
        assert!(game.try_rotate(1));
        assert_eq!(game.active.rot, 1);
        assert!(game.last_was_rotation);
        // Lateral move resets the rotation flag
        game.try_move(0, 1);
        assert!(!game.last_was_rotation);
    }

    #[test]
    fn detect_tspin_requires_t_piece() {
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
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
        let mut game = Game::new(Mode::Marathon, 0);
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
