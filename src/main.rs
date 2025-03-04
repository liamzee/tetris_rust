use crossterm::{event, terminal};
use rand::Rng;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

const SCREENSIZE: [isize; 2] = [20, 30];
const FPS: u64 = 30;
const FPSDELAY: u64 = 1000 / FPS;
const UPDATEDELAY: u64 = 1000;

struct GameState {
    blocks_on_board: Vec<Vec<bool>>,
    cursor: Cursor,
    block_orientation: Direction,
    block_type: Block,
}

struct Cursor {
    x: isize,
    y: isize,
}

#[derive(Eq, PartialEq)]
enum Direction {
    Up,
    Left,
    Right,
    Down,
}

#[derive(Clone)]
enum Block {
    Block2x2,
    LeftL,
    RightL,
    LightningUp,
    LightningDown,
    Line,
    Prod,
}

impl Block {
    ///Problematic due to manual entry of rotated block types.
    ///Better to split into internal functions and have it handled that way.
    fn realize(&self, direction: &Direction, cursor: &Cursor) -> Vec<(isize, isize)> {
        let mut template: Vec<(isize, isize)> = match self {
            Block::Block2x2 => [(0, 1), (1, 1), (0, 0), (1, 0)],
            Block::LeftL => [(0, 1), (0, 0), (0, -1), (1, -1)],
            Block::RightL => [(1, 1), (1, 0), (0, -1), (1, -1)],
            Block::LightningUp => [(1, 1), (0, 0), (1, 0), (0, -1)],
            Block::LightningDown => [(0, 1), (0, 0), (1, 0), (1, -1)],
            Block::Line => [(0, 1), (0, 0), (0, -1), (0, -2)],
            Block::Prod => [(0, 1), (-1, 0), (0, 0), (1, 0)],
        }
        .to_vec();

        let rotations = match direction {
            Direction::Up => 0,
            Direction::Right => 1,
            Direction::Down => 2,
            Direction::Left => 3,
        };

        fn rotate(arr: &mut Vec<(isize, isize)>) -> () {
            for count in 0..arr.len() {
                arr[count] = (-arr[count].1, arr[count].0)
            }
        }

        for _ in 0..rotations {
            rotate(&mut template);
        }

        let (x, y) = (cursor.x, cursor.y);

        for count in 0..template.len() {
            template[count] = (x + template[count].0, y + template[count].1);
        }

        template
    }
}

impl GameState {
    /// Generic new for GameState, calls new_block to generate the random block.
    fn new() -> GameState {
        let mut new_gamestate = GameState {
            blocks_on_board: vec![vec![false; SCREENSIZE[0] as usize]; SCREENSIZE[1] as usize],
            cursor: Cursor { x: 0, y: 0 },
            block_orientation: Direction::Up,
            block_type: Block::Block2x2,
        };
        new_gamestate.reset_cursor();
        new_gamestate.new_block();
        new_gamestate
    }

    /// Uses packaged data within struct to generate a string.
    /// "\r\n" permits function even while raw mode is on.
    fn create_string(&self) -> String {
        let active_item = self
            .block_type
            .realize(&self.block_orientation, &self.cursor);
        let mut return_string = String::new();

        for y_elem in (0..SCREENSIZE[1]).rev() {
            for x_elem in 0..SCREENSIZE[0] {
                return_string += if self.blocks_on_board[y_elem as usize][x_elem as usize] {
                    "X"
                } else if active_item.contains(&(x_elem, y_elem)) {
                    "O"
                } else {
                    " "
                };
            }
            return_string += "\r\n";
        }

        return_string
    }

    /// Generates a string, then prints it.
    fn display(&self) -> () {
        let _ = crossterm::execute!(io::stdout(), terminal::Clear(terminal::ClearType::All));
        println!("{}", self.create_string());
    }

    /// Does not return true on out of bounds. Use GameState::out_of_bounds instead.
    fn has_overlap(&self, direction: &Direction, cursor: &Cursor) -> bool {
        self.block_type
            .realize(&direction, &cursor)
            .into_iter()
            .any(|(x, y)| {
                ![x < 0, y < 0, x >= SCREENSIZE[0], y >= SCREENSIZE[1]].contains(&true)
                    && self.blocks_on_board[y as usize][x as usize]
            })
    }

    /// Helper for clearing a line on the board.
    fn clear_line(&mut self, line: &usize) -> () {
        self.blocks_on_board.remove(*line);
        self.blocks_on_board
            .push(vec![false; SCREENSIZE[0] as usize]);
    }

    /// Rotates clockwise. Does a bounds check before it rotates.
    fn rotate(&mut self) -> () {
        let new_direction = match self.block_orientation {
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
        };
        if self.out_of_bounds(&new_direction, &(self.cursor))
            || self.has_overlap(&new_direction, &(self.cursor))
        {
            return;
        } else {
            self.block_orientation = new_direction
        };
    }

    /// Main updater for game board.
    fn move_blocks(&mut self, direction: &Direction) -> () {
        if *direction == Direction::Up {
            return self.rotate();
        };
        let new_cursor = {
            let Cursor { x, y } = self.cursor;
            match *direction {
                Direction::Up => panic_gracefully(),
                Direction::Left => Cursor {
                    x: x - 1,
                    ..self.cursor
                },
                Direction::Right => Cursor {
                    x: x + 1,
                    ..self.cursor
                },
                Direction::Down => Cursor {
                    y: y - 1,
                    ..self.cursor
                },
            }
        };

        if !self.out_of_bounds(&(self.block_orientation), &new_cursor)
            && !self.has_overlap(&(self.block_orientation), &new_cursor)
        {
            self.cursor = new_cursor;
        } else if *direction == Direction::Down {
            self.adhere_blocks(); // this is passing out of bounds cases to adhere_blocks() for
            // handling.
        };
    }

    /// Checks whether the cursor creates an out of bounds element.
    fn out_of_bounds(&self, direction: &Direction, cursor: &Cursor) -> bool {
        (self.block_type.realize(&direction, &cursor))
            .into_iter()
            .any(|(x, y)| [x < 0, x >= SCREENSIZE[0], y < 0, y >= SCREENSIZE[1]].contains(&true))
    }

    /// Game overs if any element exceeds Y bound (represents overfilled
    /// tetris board), and panics if somehow an X is out of bounds or Y is
    /// below 0.
    fn adhere_blocks(&mut self) -> () {
        let new_blocks = self
            .block_type
            .realize(&(self.block_orientation), &(self.cursor));
        for (x, y) in new_blocks {
            if y >= SCREENSIZE[1] {
                self.game_over()
            };
            if [y < 0, x < 0, x >= SCREENSIZE[1]].iter().any(|a| *a) {
                panic_gracefully();
            };
            self.blocks_on_board[y as usize][x as usize] = true;
        }

        self.remove_full_lines();
        self.reset_cursor();
        self.new_block();
    }

    /// Checks for and removes full lines from the game board.
    fn remove_full_lines(&mut self) -> () {
        let mut count: usize = 0;

        while count < self.blocks_on_board.len() {
            if self.blocks_on_board[count].iter().all(|a| *a) {
                self.clear_line(&count);
                continue;
            } else {
                count += 1
            };
        }
    }

    /// Reinitializer for cursor and block orientation.
    fn reset_cursor(&mut self) -> () {
        self.cursor.y = SCREENSIZE[1] - 1;
        self.cursor.x = SCREENSIZE[0] / 2;
        self.block_orientation = Direction::Up;
    }

    /// Randomly selects a new block.
    fn new_block(&mut self) -> () {
        self.block_type = [
            Block::Block2x2,
            Block::LeftL,
            Block::RightL,
            Block::LightningUp,
            Block::LightningDown,
            Block::Line,
            Block::Prod,
        ]
        .into_iter()
        .nth(rand::rng().random_range(0..7))
        .unwrap();
    }

    /// Game over.
    fn game_over(&self) -> () {
        let _ = crossterm::terminal::disable_raw_mode();
        self.display();
        println!("Game over!");
        std::process::exit(0);
    }
}

/// Helper function for safe exits due to raw mode being on.
fn panic_gracefully() -> ! {
    let _ = crossterm::terminal::disable_raw_mode();
    println!("internal panic triggered");
    std::process::exit(1);
}

/// Main.
fn main() -> () {
    let mut game = GameState::new();
    let _ = terminal::enable_raw_mode();
    game_loop(&mut game);
    let _ = terminal::disable_raw_mode();
}

/// Main loop.
fn game_loop(game: &mut GameState) -> () {
    let mut update_clock = Instant::now();
    let mut display_clock = update_clock.clone();

    loop {
        game.display();

        key_listener(game);

        let update_elapsed = update_clock.elapsed();

        if update_elapsed > Duration::from_millis(UPDATEDELAY) {
            update_clock += Duration::from_millis(UPDATEDELAY);
            game.move_blocks(&Direction::Down);
        }

        let screen_elapsed = display_clock.elapsed();

        if screen_elapsed < Duration::from_millis(FPSDELAY) {
            thread::sleep(Duration::from_millis(FPSDELAY) - screen_elapsed);
        } else {
            display_clock += Duration::from_millis(FPSDELAY)
        };
    }
}

/// Key listener code.
fn key_listener(game: &mut GameState) -> () {
    while let Ok(true) = event::poll(Duration::from_secs(0)) {
        let Ok(event::Event::Key(pressed)) = event::read() else {
            break;
        };
        match pressed.code {
            event::KeyCode::Up => {
                game.move_blocks(&Direction::Up);
            }
            event::KeyCode::Left => {
                game.move_blocks(&Direction::Left);
            }
            event::KeyCode::Right => {
                game.move_blocks(&Direction::Right);
            }
            event::KeyCode::Down => {
                game.move_blocks(&Direction::Down);
            }
            event::KeyCode::Char('c') => {
                if pressed.modifiers.contains(event::KeyModifiers::CONTROL) {
                    panic_gracefully()
                };
            }
            _ => break,
        };
    }
}
