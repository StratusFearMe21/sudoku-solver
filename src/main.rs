/*
 * My very cool sudoku program
 */

use std::{
    io::stdout,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    event, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dashmap::{DashMap, DashSet};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

// How often tui checks for keypresses
const TICKRATE: Duration = Duration::from_millis(250);

// We use this macro to draw to the terminal
macro_rules! draw {
    ($a:expr, $b:expr) => {
        $a.draw(|f| {
            let text = vec![
                Spans::from(format!("{:?}", $b[0])),
                Spans::from(format!("{:?}", $b[1])),
                Spans::from(format!("{:?}", $b[2])),
                Spans::from(format!("{:?}", $b[3])),
                Spans::from(format!("{:?}", $b[4])),
                Spans::from(format!("{:?}", $b[5])),
                Spans::from(format!("{:?}", $b[6])),
                Spans::from(format!("{:?}", $b[7])),
                Spans::from(format!("{:?}", $b[8])),
            ];

            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(5)
                .constraints([Constraint::Length(11), Constraint::Min(0)].as_ref())
                .split(size);

            let block = tui::widgets::Block::default()
                .borders(tui::widgets::Borders::ALL)
                .title("Sudoku Solver")
                .border_type(tui::widgets::BorderType::Rounded);
            f.render_widget(block, size);
            let create_block = |title| {
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::White).fg(Color::Black))
                    .title(Span::styled(
                        title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
            };
            let paragraph = Paragraph::new(text.clone())
                .style(Style::default().bg(Color::White).fg(Color::Black))
                .block(create_block("Board"))
                .alignment(Alignment::Center);
            f.render_widget(paragraph, chunks[0]);
            let paragraph = Paragraph::new(vec![
                Spans::from("Hit enter to solve puzzle"),
                Spans::from("- Make sure that the puzzle does not require guessing (programs such as KSudoku can tell you this)"),
                Spans::from("- Make sure that the numbers 1-9 are present in the puzzle"),
                Spans::from("- Make sure all numbers in puzzle are correct"),
            ])
            .style(Style::default().bg(Color::White).fg(Color::Black))
            .block(create_block("Instructions"))
            .alignment(Alignment::Center);
            f.render_widget(paragraph, chunks[1]);
        })
        .unwrap();
    };
}

fn main() {
    // Initializiation
    enable_raw_mode().unwrap();

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    // Keyboard listener
    thread::spawn(move || loop {
        let mut last_tick = Instant::now();
        if event::poll(
            TICKRATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0)),
        )
        .unwrap()
        {
            if let event::Event::Key(key) = event::read().unwrap() {
                tx.send(key).unwrap();
            }
        }
        if last_tick.elapsed() >= TICKRATE {
            last_tick = Instant::now();
        }
    });

    terminal.clear().unwrap();

    // The sudoku board
    let mut board: [[u8; 9]; 9] = serde_json::from_str(
        &std::fs::read_to_string("./puzzle.json").unwrap_or_else(|_| {
            std::fs::write(
                "./puzzle.json",
                serde_json::to_string(&[[0; 9]; 9]).unwrap(),
            )
            .unwrap();
            std::fs::read_to_string("./puzzle.json").unwrap()
        }),
    )
    .unwrap();
    let mut boardbools: [[bool; 9]; 9] = [[false; 9]; 9];

    for (i, j) in board.iter().enumerate() {
        for (x, y) in j.iter().enumerate() {
            if y.clone() != 0 {
                boardbools[i][x] = true;
            }
        }
    }

    // Used for editing the board
    let mut index: (usize, usize) = (0, 0);

    draw!(terminal, board);

    loop {
        // Listen for key presses
        match rx.recv().unwrap() {
            event => match event.code {
                event::KeyCode::Char('q') | event::KeyCode::Enter => {
                    // Start solving the given board
                    break;
                }
                event::KeyCode::Char(c) => {
                    let num: u8 = c.to_digit(10).unwrap_or(0) as u8;
                    board[index.0][index.1] = num;
                    boardbools[index.0][index.1] = true;
                    index.1 += 1;
                    if index.1 == 9 {
                        index.0 += 1;
                        index.1 = 0;
                    }
                    draw!(terminal, board);
                }
                event::KeyCode::Backspace => {
                    index.1 -= 1;
                    board[index.0][index.1] = 0;
                    boardbools[index.0][index.1] = false;
                    draw!(terminal, board);
                }
                _ => {}
            },
        };
    }

    // Whether the algorithm should keep solving
    let mut recurse = true;

    let mut available: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    while recurse {
        // What to add to the board after an iteration
        let additions: DashSet<((usize, usize), u8)> = DashSet::new();

        // Iterate for each number possible in sudoku
        available.clone().into_par_iter().for_each(|numberon| {
            // Positions of number being processed
            let positions: DashSet<(usize, usize)> = DashSet::new();

            // Determine what "positions" should be
            board.par_iter().enumerate().for_each(|(rownum, row)| {
                let positionsraw = row.par_iter().positions(|&val| val == numberon);
                positionsraw.into_par_iter().for_each(|val| {
                    positions.insert((rownum, val));
                });
            });

            // Converts DashSet into Vec for paralell iteration
            let finalpos: Vec<(usize, usize)> = positions.into_iter().collect();

            // Positions where are number in processing cannot go
            let impossiblepos: DashSet<(usize, usize)> = DashSet::new();

            // Determines which positions are impossible for the number in processing can go.
            finalpos.into_par_iter().for_each(|position| {
                // Determines which operation is being done on the number
                (0..=2).into_par_iter().for_each(|state| match state {
                    // Fill row of number with impossible positions
                    0 => board[position.0]
                        .par_iter()
                        .enumerate()
                        .for_each(|(val, &b)| {
                            if b == 0 {
                                impossiblepos.insert((position.0, val));
                            }
                        }),

                    // Fill collumn of number with impossible positions
                    1 => board.par_iter().enumerate().for_each(|(val, b)| {
                        if b[position.1] == 0 {
                            impossiblepos.insert((val, position.1));
                        }
                    }),

                    // Fill block of number with impossible positions
                    2 => {
                        let finalblock: (usize, usize) = (
                            match position.0 {
                                0..=2 => 1,
                                3..=5 => 2,
                                _ => 3,
                            },
                            match position.1 {
                                0..=2 => 1,
                                3..=5 => 2,
                                _ => 3,
                            },
                        );
                        match finalblock {
                            (1, 1) => {
                                (0..=2).into_par_iter().for_each(|r| {
                                    (0..=2).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (1, 2) => {
                                (0..=2).into_par_iter().for_each(|r| {
                                    (3..=5).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (1, 3) => {
                                (0..=2).into_par_iter().for_each(|r| {
                                    (6..=8).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (2, 1) => {
                                (3..=5).into_par_iter().for_each(|r| {
                                    (0..=2).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (2, 2) => {
                                (3..=5).into_par_iter().for_each(|r| {
                                    (3..=5).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (2, 3) => {
                                (3..=5).into_par_iter().for_each(|r| {
                                    (6..=8).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (3, 1) => {
                                (6..=8).into_par_iter().for_each(|r| {
                                    (0..=2).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (3, 2) => {
                                (6..=8).into_par_iter().for_each(|r| {
                                    (3..=5).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            (3, 3) => {
                                (6..=8).into_par_iter().for_each(|r| {
                                    (6..=8).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                })
            });
            let impossibleposvec: Vec<(usize, usize)> = impossiblepos.into_iter().collect();

            (0..=1).into_par_iter().for_each(|state| match state {
                0 => {
                    (0..9).into_par_iter().for_each(|row: i32| {
                        let mut rowabs: [bool; 9] = [false; 9];
                        impossibleposvec
                            .iter()
                            .filter(|i| i.0 == row as usize)
                            .for_each(|pos| {
                                rowabs[pos.1] = true;
                            });
                        boardbools[row as usize]
                            .iter()
                            .enumerate()
                            .for_each(|(i, index)| {
                                if index.clone() {
                                    rowabs[i] = true;
                                }
                            });
                        let numtrue = rowabs.into_par_iter().positions(|x| x == &true).count();
                        if numtrue == 8 {
                            additions.insert((
                                (
                                    row as usize,
                                    rowabs.into_par_iter().position_any(|x| !x).unwrap(),
                                ),
                                numberon,
                            ));
                        }
                    });
                }
                1 => {
                    (0..9).into_par_iter().for_each(|column: i32| {
                        let mut rowabs: [bool; 9] = [false; 9];
                        impossibleposvec
                            .iter()
                            .filter(|i| i.1 == column as usize)
                            .for_each(|pos| {
                                rowabs[pos.0] = true;
                            });
                        boardbools.iter().enumerate().for_each(|(i, list)| {
                            if list[column as usize] {
                                rowabs[i] = true;
                            }
                        });
                        let numtrue = rowabs.into_par_iter().positions(|x| x == &true).count();
                        if numtrue == 8 {
                            additions.insert((
                                (
                                    rowabs.into_par_iter().position_any(|x| !x).unwrap(),
                                    column as usize,
                                ),
                                numberon,
                            ));
                        }
                    });
                }
                _ => {}
            });
        });
        if additions.iter().count() == 0 {
            recurse = false;
        } else {
            additions.iter().for_each(|i| {
                board[i.0 .0][i.0 .1] = i.1;
                boardbools[i.0 .0][i.0 .1] = true;
            });
        }
    }
    draw!(terminal, board);

    loop {
        match rx.recv().unwrap() {
            event => match event.code {
                event::KeyCode::Char('q') | event::KeyCode::Enter => {
                    disable_raw_mode().unwrap();
                    execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
                    terminal.show_cursor().unwrap();
                    break;
                }
                event::KeyCode::Char(c) => {
                    let num: u8 = c.to_digit(10).unwrap_or(0) as u8;
                    board[index.0][index.1] = num;
                    index.1 += 1;
                    if index.1 == 9 {
                        index.0 += 1;
                        index.1 = 0;
                    }
                    draw!(terminal, board);
                }
                event::KeyCode::Backspace => {
                    index.1 -= 1;
                    board[index.0][index.1] = 0;
                    draw!(terminal, board);
                }
                _ => {}
            },
        };
    }
}
