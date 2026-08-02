/*
 *  Snake game for Seminar BSE.
 *  Abdulrahman Al Hamidi
 *  3034747
 */

use alloc::format;
use alloc::vec::Vec;
use core::cmp::{max, min};

use crate::device::framebuffer::{self, color, Framebuffer};
use crate::device::key::Scancode;
use crate::device::keyboard::keyboard_buffer;
use crate::device::pit;
use crate::device::terminal::terminal;

const BOARD_WIDTH: usize = 32;
const BOARD_HEIGHT: usize = 24;
const INITIAL_LENGTH: usize = 5;
const INITIAL_STEP_MS: usize = 135;
const MIN_STEP_MS: usize = 55;
const SPEEDUP_MS: usize = 5;

const BACKGROUND: u32 = color(7, 12, 18);
const PANEL: u32 = color(14, 24, 32);
const BOARD_DARK: u32 = color(18, 31, 25);
const BOARD_LIGHT: u32 = color(21, 38, 29);
const BORDER: u32 = color(66, 105, 76);
const SNAKE_HEAD: u32 = color(103, 230, 112);
const SNAKE_BODY: u32 = color(48, 180, 73);
const SNAKE_INNER: u32 = color(68, 205, 89);
const FOOD: u32 = color(225, 55, 62);
const FOOD_LIGHT: u32 = color(255, 118, 82);
const TEXT: u32 = color(220, 235, 224);
const TEXT_MUTED: u32 = color(142, 165, 150);
const ACCENT: u32 = color(94, 220, 112);
const BLACK: u32 = framebuffer::BLACK;

#[derive(Copy, Clone, PartialEq, Eq)]
struct Position {
    x: usize,
    y: usize,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn is_opposite(self, other: Direction) -> bool {
        matches!(
            (self, other),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        )
    }
}

#[derive(Copy, Clone)]
struct Layout {
    screen_width: usize,
    screen_height: usize,
    board_x: usize,
    board_y: usize,
    cell_size: usize,
    board_pixel_width: usize,
    board_pixel_height: usize,
}

impl Layout {
    fn new(screen_width: usize, screen_height: usize) -> Layout {
        let horizontal_space = screen_width.saturating_sub(48);
        let vertical_space = screen_height.saturating_sub(150);
        let by_width = horizontal_space / BOARD_WIDTH;
        let by_height = vertical_space / BOARD_HEIGHT;
        let cell_size = max(6, min(20, min(by_width, by_height)));
        let board_pixel_width = BOARD_WIDTH * cell_size;
        let board_pixel_height = BOARD_HEIGHT * cell_size;
        let board_x = screen_width.saturating_sub(board_pixel_width) / 2;
        let board_y = 84;

        Layout {
            screen_width,
            screen_height,
            board_x,
            board_y,
            cell_size,
            board_pixel_width,
            board_pixel_height,
        }
    }
}

struct Game {
    body: Vec<Position>,
    direction: Direction,
    queued_direction: Direction,
    turn_queued: bool,
    food: Position,
    score: usize,
    step_ms: usize,
    rng_state: u64,
    won: bool,
}

impl Game {
    fn new(seed: u64) -> Game {
        let center_x = BOARD_WIDTH / 2;
        let center_y = BOARD_HEIGHT / 2;
        let mut body = Vec::with_capacity(BOARD_WIDTH * BOARD_HEIGHT);

        for index in 0..INITIAL_LENGTH {
            body.push(Position {
                x: center_x - index,
                y: center_y,
            });
        }

        let mut game = Game {
            body,
            direction: Direction::Right,
            queued_direction: Direction::Right,
            turn_queued: false,
            food: Position {x: 0, y: 0 },
            score: 0,
            step_ms: INITIAL_STEP_MS,
            rng_state: seed | 1,
            won: false,
        };

        game.place_food();
        game
    }

    fn queue_direction(&mut self, direction: Direction) {
        if self.turn_queued || direction.is_opposite(self.direction) {
            return;
        }

        self.queued_direction = direction;
        self.turn_queued = true;
    }

    fn update(&mut self) -> bool {
        self.direction = self.queued_direction;
        self.turn_queued = false;

        let head = self.body[0];
        let new_head = match self.direction {
            Direction::Up => {
                if head.y == 0 {
                    return false;
                }
                Position { x: head.x, y: head.y - 1 }
            }
            Direction::Down => {
                if head.y + 1 >= BOARD_HEIGHT {
                    return false;
                }
                Position { x: head.x, y: head.y + 1 }
            }
            Direction::Left => {
                if head.x == 0 {
                    return false;
                }
                Position { x: head.x - 1, y: head.y }
            }
            Direction::Right => {
                if head.x + 1 >= BOARD_WIDTH {
                    return false;
                }
                Position { x: head.x + 1, y: head.y }
            }
        };

        let grows = new_head == self.food;
        let checked_length = if grows {
            self.body.len()
        } else {
            self.body.len().saturating_sub(1)
        };

        if self.body[..checked_length].iter().any(|part| *part == new_head) {
            return false;
        }

        self.body.insert(0, new_head);

        if grows {
            self.score += 10;
            self.step_ms = max(MIN_STEP_MS, self.step_ms.saturating_sub(SPEEDUP_MS));
            self.place_food();
        } else {
            self.body.pop();
        }

        true
    }

    fn place_food(&mut self) {
        if self.body.len() >= BOARD_WIDTH * BOARD_HEIGHT {
            self.won = true;
            return;
        }

        for _ in 0..(BOARD_WIDTH * BOARD_HEIGHT * 2) {
            let candidate = Position {
                x: self.random_usize(BOARD_WIDTH),
                y: self.random_usize(BOARD_HEIGHT),
            };

            if !self.body.iter().any(|part| *part == candidate) {
                self.food = candidate;
                return;
            }
        }

        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                let candidate = Position { x, y };
                if !self.body.iter().any(|part| *part == candidate) {
                    self.food = candidate;
                    return;
                }
            }
        }

        self.won = true;
    }

    fn random_usize(&mut self, limit: usize) -> usize {
        let mut value = self.rng_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.rng_state = value;
        (value as usize) % limit
    }
}

enum RunResult {
    Restart(usize),
    Exit(usize),
}

pub fn play() {
    flush_keyboard_buffer();

    let mut high_score = 0;
    if !show_start_screen(high_score) {
        show_exit_screen();
        return;
    }

    loop {
        match run_game(high_score) {
            RunResult::Restart(score) => {
                high_score = max(high_score, score);
            }
            RunResult::Exit(score) => {
                high_score = max(high_score, score);
                show_exit_screen_with_score(high_score);
                return;
            }
        }
    }
}

fn run_game(high_score: usize) -> RunResult {
    flush_keyboard_buffer();

    let seed = (pit::system_time() as u64)
        ^ ((high_score as u64) << 32)
        ^ 0x9e37_79b9_7f4a_7c15;

    let mut game = Game::new(seed);
    let layout = current_layout();
    let mut last_step = pit::system_time();

    render_game(&game, layout, high_score);

    loop {
        while let Some(event) = keyboard_buffer().pop_key_event() {
            if !event.pressed() {
                continue;
            }

            match event.scancode() {
                Some(Scancode::Up) | Some(Scancode::W) => {
                    game.queue_direction(Direction::Up);
                }
                Some(Scancode::Down) | Some(Scancode::S) => {
                    game.queue_direction(Direction::Down);
                }
                Some(Scancode::Left) | Some(Scancode::A) => {
                    game.queue_direction(Direction::Left);
                }
                Some(Scancode::Right) | Some(Scancode::D) => {
                    game.queue_direction(Direction::Right);
                }
                Some(Scancode::Escape) => {
                    return RunResult::Exit(game.score);
                }
                _ => {}
            }
        }

        let now = pit::system_time();

        if now.wrapping_sub(last_step) >= game.step_ms {
            last_step = now;

            let old_head = game.body[0];
            let old_tail = *game
                .body
                .last()
                .expect("Snake must always contain at least one segment");
            let old_length = game.body.len();

            if !game.update() || game.won {
                let final_high_score = max(high_score, game.score);

                render_game(&game, layout, final_high_score);

                return wait_after_game(
                    &game,
                    layout,
                    final_high_score,
                );
            }

            let grew = game.body.len() > old_length;

            render_game_step(
                &game,
                layout,
                old_head,
                old_tail,
                grew,
                max(high_score, game.score),
            );
        }

        pit::wait(1);
    }
}

fn wait_after_game(game: &Game, layout: Layout, high_score: usize) -> RunResult {
    render_game_over(game, layout, high_score);
    flush_keyboard_buffer();

    loop {
        while let Some(event) = keyboard_buffer().pop_key_event() {
            if !event.pressed() {
                continue;
            }

            match event.scancode() {
                Some(Scancode::Enter) | Some(Scancode::R) => {
                    return RunResult::Restart(game.score);
                }
                Some(Scancode::Escape) => return RunResult::Exit(game.score),
                _ => {}
            }
        }

        pit::wait(5);
    }
}

fn show_start_screen(high_score: usize) -> bool {
    let terminal_guard = terminal().lock();
    let mut framebuffer = terminal_guard.framebuffer().lock();
    let layout = Layout::new(framebuffer.width(), framebuffer.height());

    framebuffer.clear();
    fill_rect(&mut framebuffer, 0, 0, layout.screen_width, layout.screen_height, BACKGROUND);

    draw_centered(&mut framebuffer, 86, "S N A K E", ACCENT, BACKGROUND);
    draw_centered(
        &mut framebuffer,
        120,
        "Snake game implented by Abdul_Rahman",
        TEXT_MUTED,
        BACKGROUND,
    );

    let preview_y = 178;
    let preview_cell = 20;
    let preview_width = 9 * preview_cell;
    let preview_x = layout.screen_width.saturating_sub(preview_width) / 2;
    for index in 0..7 {
        let x = preview_x + index * preview_cell;
        let color = if index == 6 { SNAKE_HEAD } else { SNAKE_BODY };
        fill_rect(&mut framebuffer, x + 2, preview_y + 2, preview_cell - 4, preview_cell - 4, color);
    }
    fill_rect(
        &mut framebuffer,
        preview_x + 8 * preview_cell + 4,
        preview_y + 4,
        preview_cell - 8,
        preview_cell - 8,
        FOOD,
    );

    draw_centered(
        &mut framebuffer,
        255,
        "Controls: Arrwo keys or W A S D",
        TEXT,
        BACKGROUND,
    );
    draw_centered(
        &mut framebuffer,
        282,
        "ESC: Exit game",
        TEXT_MUTED,
        BACKGROUND,
    );

    if high_score > 0 {
        let high_score_text = format!("High Score: {}", high_score);
        draw_centered(&mut framebuffer, 325, &high_score_text, FOOD_LIGHT, BACKGROUND);
    }

    draw_centered(
        &mut framebuffer,
        380,
        "ENTER to start",
        ACCENT,
        BACKGROUND,
    );

    drop(framebuffer);
    drop(terminal_guard);

    flush_keyboard_buffer();
    loop {
        while let Some(event) = keyboard_buffer().pop_key_event() {
            if !event.pressed() {
                continue;
            }

            match event.scancode() {
                Some(Scancode::Enter) => return true,
                Some(Scancode::Escape) => return false,
                _ => {}
            }
        }
        pit::wait(5);
    }
}

fn render_game(game: &Game, layout: Layout, high_score: usize) {
    let terminal_guard = terminal().lock();
    let mut framebuffer = terminal_guard.framebuffer().lock();

    fill_rect(
        &mut framebuffer,
        0,
        0,
        layout.screen_width,
        layout.screen_height,
        BACKGROUND,
    );

    draw_status_bar(&mut framebuffer, game, high_score);
    draw_board_border(&mut framebuffer, layout);

    for y in 0..BOARD_HEIGHT {
        for x in 0..BOARD_WIDTH {
            draw_board_cell(
                &mut framebuffer,
                layout,
                Position { x, y },
            );
        }
    }

    draw_food(&mut framebuffer, layout, game.food);

    for (index, part) in game.body.iter().enumerate().rev() {
        draw_snake_part(
            &mut framebuffer,
            layout,
            *part,
            index == 0,
            game.direction,
        );
    }

    let help_y = min(
        layout.screen_height.saturating_sub(framebuffer::CHAR_HEIGHT + 12),
        layout.board_y + layout.board_pixel_height + 22,
    );

    draw_centered(
        &mut framebuffer,
        help_y,
        "Arrow keys/WASD: Move   ESC: Exit",
        TEXT_MUTED,
        BACKGROUND,
    );
}

fn render_game_step(
    game: &Game,
    layout: Layout,
    old_head: Position,
    old_tail: Position,
    grew: bool,
    high_score: usize,
) {
    let terminal_guard = terminal().lock();
    let mut framebuffer = terminal_guard.framebuffer().lock();

    if !grew {
        draw_board_cell(&mut framebuffer, layout, old_tail);
    }

    draw_snake_part(
        &mut framebuffer,
        layout,
        old_head,
        false,
        game.direction,
    );

    draw_snake_part(
        &mut framebuffer,
        layout,
        game.body[0],
        true,
        game.direction,
    );

    if grew {
        draw_food(&mut framebuffer, layout, game.food);
        draw_status_bar(&mut framebuffer, game, high_score);
    }
}

fn draw_status_bar(
    framebuffer: &mut Framebuffer,
    game: &Game,
    high_score: usize,
) {
    let screen_width = framebuffer.width();

    fill_rect(
        framebuffer,
        0,
        0,
        screen_width,
        70,
        BACKGROUND,
    );

    framebuffer.draw_str(
        "SNAKE",
        24,
        20,
        ACCENT,
        BACKGROUND,
    );

    let score = format!("Score: {}", game.score);
    framebuffer.draw_str(
        &score,
        150,
        20,
        TEXT,
        BACKGROUND,
    );

    let high_score_text = format!("High Score: {}", high_score);
    framebuffer.draw_str(
        &high_score_text,
        310,
        20,
        TEXT_MUTED,
        BACKGROUND,
    );

    let speed = format!("Speed: {}", speed_level(game.step_ms));
    framebuffer.draw_str(
        &speed,
        500,
        20,
        FOOD_LIGHT,
        BACKGROUND,
    );
}

fn draw_board_border(framebuffer: &mut Framebuffer, layout: Layout) {
    let x = layout.board_x;
    let y = layout.board_y;
    let width = layout.board_pixel_width;
    let height = layout.board_pixel_height;
    let border_size = 4;

    // Top
    fill_rect(
        framebuffer,
        x.saturating_sub(border_size),
        y.saturating_sub(border_size),
        width + border_size * 2,
        border_size,
        BORDER,
    );

    // Bottom
    fill_rect(
        framebuffer,
        x.saturating_sub(border_size),
        y + height,
        width + border_size * 2,
        border_size,
        BORDER,
    );

    // Left
    fill_rect(
        framebuffer,
        x.saturating_sub(border_size),
        y,
        border_size,
        height,
        BORDER,
    );

    // Right
    fill_rect(
        framebuffer,
        x + width,
        y,
        border_size,
        height,
        BORDER,
    );
}

fn draw_board_cell(
    framebuffer: &mut Framebuffer,
    layout: Layout,
    position: Position,
) {
    let tile_color = if (position.x + position.y) % 2 == 0 {
        BOARD_DARK
    } else {
        BOARD_LIGHT
    };

    fill_rect(
        framebuffer,
        layout.board_x + position.x * layout.cell_size,
        layout.board_y + position.y * layout.cell_size,
        layout.cell_size,
        layout.cell_size,
        tile_color,
    );
}

fn render_game_over(game: &Game, layout: Layout, high_score: usize) {
    let terminal_guard = terminal().lock();
    let mut framebuffer = terminal_guard.framebuffer().lock();
    let title = if game.won { "YOU WIN!" } else { "GAME OVER" };
    let subtitle = format!("Score: {}   High Score: {}", game.score, high_score);

    draw_overlay(&mut framebuffer, layout, title, &subtitle);

    let hint_y = layout.board_y + layout.board_pixel_height / 2 + 52;
    draw_centered(
        &mut framebuffer,
        hint_y,
        "ENTER/R: Restart    ESC: Exit",
        ACCENT,
        PANEL,
    );
}

fn draw_overlay(framebuffer: &mut Framebuffer, layout: Layout, title: &str, subtitle: &str) {
    let panel_width = min(520, layout.screen_width.saturating_sub(40));
    let panel_height = 150;
    let panel_x = layout.screen_width.saturating_sub(panel_width) / 2;
    let panel_y = layout.board_y + layout.board_pixel_height.saturating_sub(panel_height) / 2;

    fill_rect(
        framebuffer,
        panel_x.saturating_sub(4),
        panel_y.saturating_sub(4),
        panel_width + 8,
        panel_height + 8,
        BORDER,
    );
    fill_rect(framebuffer, panel_x, panel_y, panel_width, panel_height, PANEL);
    draw_centered(framebuffer, panel_y + 34, title, FOOD_LIGHT, PANEL);
    draw_centered(framebuffer, panel_y + 76, subtitle, TEXT, PANEL);
}

fn draw_snake_part(
    framebuffer: &mut Framebuffer,
    layout: Layout,
    position: Position,
    head: bool,
    direction: Direction,
) {
    let x = layout.board_x + position.x * layout.cell_size;
    let y = layout.board_y + position.y * layout.cell_size;
    let margin = max(1, layout.cell_size / 8);
    let size = layout.cell_size.saturating_sub(margin * 2);
    let body_color = if head { SNAKE_HEAD } else { SNAKE_BODY };

    fill_rect(framebuffer, x + margin, y + margin, size, size, body_color);

    if !head {
        let inner_margin = max(2, layout.cell_size / 4);
        let inner_size = layout.cell_size.saturating_sub(inner_margin * 2);
        if inner_size > 0 {
            fill_rect(
                framebuffer,
                x + inner_margin,
                y + inner_margin,
                inner_size,
                inner_size,
                SNAKE_INNER,
            );
        }
        return;
    }

    if layout.cell_size < 10 {
        return;
    }

    let eye = max(2, layout.cell_size / 7);
    let near = margin + 2;
    let far = layout.cell_size.saturating_sub(margin + eye + 2);
    let middle_a = layout.cell_size / 3;
    let middle_b = layout.cell_size.saturating_sub(layout.cell_size / 3 + eye);

    match direction {
        Direction::Up => {
            fill_rect(framebuffer, x + middle_a, y + near, eye, eye, BLACK);
            fill_rect(framebuffer, x + middle_b, y + near, eye, eye, BLACK);
        }
        Direction::Down => {
            fill_rect(framebuffer, x + middle_a, y + far, eye, eye, BLACK);
            fill_rect(framebuffer, x + middle_b, y + far, eye, eye, BLACK);
        }
        Direction::Left => {
            fill_rect(framebuffer, x + near, y + middle_a, eye, eye, BLACK);
            fill_rect(framebuffer, x + near, y + middle_b, eye, eye, BLACK);
        }
        Direction::Right => {
            fill_rect(framebuffer, x + far, y + middle_a, eye, eye, BLACK);
            fill_rect(framebuffer, x + far, y + middle_b, eye, eye, BLACK);
        }
    }
}

fn draw_food(framebuffer: &mut Framebuffer, layout: Layout, position: Position) {
    let x = layout.board_x + position.x * layout.cell_size;
    let y = layout.board_y + position.y * layout.cell_size;
    let margin = max(2, layout.cell_size / 5);
    let size = layout.cell_size.saturating_sub(margin * 2);

    fill_rect(framebuffer, x + margin, y + margin, size, size, FOOD);

    let highlight = max(1, layout.cell_size / 8);
    fill_rect(
        framebuffer,
        x + margin + 1,
        y + margin + 1,
        highlight,
        highlight,
        FOOD_LIGHT,
    );
}

fn current_layout() -> Layout {
    let terminal_guard = terminal().lock();
    let framebuffer = terminal_guard.framebuffer().lock();
    Layout::new(framebuffer.width(), framebuffer.height())
}

fn fill_rect(
    framebuffer: &mut Framebuffer,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
) {
    let end_x = min(framebuffer.width(), x.saturating_add(width));
    let end_y = min(framebuffer.height(), y.saturating_add(height));

    if x >= end_x || y >= end_y {
        return;
    }

    for current_y in y..end_y {
        for current_x in x..end_x {
            unsafe {
                framebuffer.draw_pixel_unchecked(current_x, current_y, color);
            }
        }
    }
}

fn draw_centered(
    framebuffer: &mut Framebuffer,
    y: usize,
    text: &str,
    foreground: u32,
    background: u32,
) {
    let character_width = framebuffer::CHAR_WIDTH + 1;
    let text_width = text.chars().count() * character_width;
    let x = framebuffer.width().saturating_sub(text_width) / 2;
    framebuffer.draw_str(text, x, y, foreground, background);
}

fn speed_level(step_ms: usize) -> usize {
    1 + (INITIAL_STEP_MS.saturating_sub(step_ms) / SPEEDUP_MS)
}

fn flush_keyboard_buffer() {
    while keyboard_buffer().pop_key_event().is_some() {}
}

fn show_exit_screen() {
    show_exit_screen_with_score(0);
}

fn show_exit_screen_with_score(high_score: usize) {
    let terminal_guard = terminal().lock();
    let mut framebuffer = terminal_guard.framebuffer().lock();
    let width = framebuffer.width();
    let height = framebuffer.height();

    fill_rect(&mut framebuffer, 0, 0, width, height, BACKGROUND);
    draw_centered(&mut framebuffer, 180, "Snake has been closed.", TEXT, BACKGROUND);

    if high_score > 0 {
        let score = format!("High Score: {}", high_score);
        draw_centered(&mut framebuffer, 220, &score, ACCENT, BACKGROUND);
    }

    draw_centered(
        &mut framebuffer,
        270,
        "Restart to play again.",
        TEXT_MUTED,
        BACKGROUND,
    );
}