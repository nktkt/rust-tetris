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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
        [Piece::I, Piece::O, Piece::T, Piece::S, Piece::Z, Piece::J, Piece::L]
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
}

impl Game {
    fn new(best_score: u32) -> Self {
        let mut bag = Bag::new();
        let piece = bag.next();
        Game {
            board: [[None; WIDTH]; HEIGHT],
            active: Active { piece, rot: 0, row: 0, col: 3 },
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
            // Lateral moves (dr == 0) count as resettable actions; downward gravity does not.
            self.after_move(dr == 0);
            true
        } else {
            false
        }
    }

    fn try_rotate(&mut self, dir: i32) -> bool {
        let new_rot = ((self.active.rot as i32 + dir).rem_euclid(4)) as usize;
        let kicks: [(i32, i32); 5] = [(0, 0), (0, -1), (0, 1), (0, -2), (0, 2)];
        for k in kicks {
            let cells =
                self.active
                    .cells_with(new_rot, self.active.row + k.0, self.active.col + k.1);
            if !self.collides(&cells) {
                self.active.rot = new_rot;
                self.active.row += k.0;
                self.active.col += k.1;
                self.after_move(true);
                return true;
            }
        }
        false
    }

    fn lock_piece(&mut self) {
        let color = self.active.piece.color();
        for (r, c) in self.active.cells() {
            if r >= 0 && r < HEIGHT as i32 && c >= 0 && c < WIDTH as i32 {
                self.board[r as usize][c as usize] = Some(color);
            }
        }
        self.clear_lines();
        self.lock_timer = None;
        self.lock_resets = 0;
        self.hold_used = false;
        self.spawn_next();
    }

    fn clear_lines(&mut self) {
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
        if cleared > 0 {
            self.lines += cleared;
            let points = match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                _ => 800,
            };
            self.score += points * self.level;
            self.level = 1 + self.lines / 10;
        }
    }

    fn spawn_next(&mut self) {
        let piece = self.bag.next();
        self.active = Active { piece, rot: 0, row: 0, col: 3 };
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
        self.active = Active { piece: new_piece, rot: 0, row: 0, col: 3 };
        self.lock_timer = None;
        self.lock_resets = 0;
        if self.collides(&self.active.cells()) {
            self.game_over = true;
        } else {
            self.after_move(false);
        }
    }

    fn ghost_row(&self) -> i32 {
        let mut r = self.active.row;
        loop {
            let cells = self.active.cells_with(self.active.rot, r + 1, self.active.col);
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
    let ghost_cells = game.active.cells_with(game.active.rot, ghost_r, game.active.col);
    let active_cells = game.active.cells();

    for r in 0..HEIGHT {
        queue!(out, cursor::MoveTo(board_x - 1, board_y + r as u16), Print("│"))?;
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
            cursor::MoveTo(board_x + (WIDTH as u16) - 9, board_y + (HEIGHT as u16) / 2 + 1),
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
