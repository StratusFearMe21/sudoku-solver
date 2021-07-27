/*
 * My very cool sudoku program
 */

#[cfg(not(windows))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use std::{
    io::stdout,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    event, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dashmap::DashSet;
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
                .constraints([Constraint::Percentage(100)].as_ref())
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

    // Used to keep track of just whether there is a number in a square.
    let mut boardbools: [[bool; 9]; 9] = [[false; 9]; 9];

    // Fill boardbools
    for (i, j) in board.iter().enumerate() {
        for (x, y) in j.iter().enumerate() {
            if y.clone() != 0 {
                boardbools[i][x] = true;
            }
        }
    }

    // Whether the algorithm should keep solving
    let mut recurse = true;

    // let mut available: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    // Time starts here
    let time = Instant::now();

    while recurse {
        // let mut numtoremove: DashSet<usize> = DashSet::new();

        // What to add to the board after an iteration
        let additions: DashSet<((usize, usize), u8)> = DashSet::new();

        // Iterate for each number possible in sudoku
        (1 as u8..=9 as u8).into_par_iter().for_each(|numberon| {
            // Positions of number being processed
            let positions: DashSet<(usize, usize)> = DashSet::new();

            // Determine what "positions" should be
            board.par_iter().enumerate().for_each(|(rownum, row)| {
                row.par_iter()
                    .positions(|&val| val == numberon)
                    .for_each(|val| {
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
                rayon::scope(|s| {
                    // Fill row of number with impossible positions
                    s.spawn(|_| {
                        board[position.0]
                            .par_iter()
                            .enumerate()
                            .for_each(|(val, &b)| {
                                if b == 0 {
                                    impossiblepos.insert((position.0, val));
                                }
                            })
                    });

                    // Fill collumn of number with impossible positions
                    s.spawn(|_| {
                        board.par_iter().enumerate().for_each(|(val, b)| {
                            if b[position.1] == 0 {
                                impossiblepos.insert((val, position.1));
                            }
                        })
                    });

                    // Fill block of number with impossible positions
                    s.spawn(|_| {
                        // Determine the block that the number is in
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

                        // Process based on block determined
                        match finalblock {
                            // Block 1
                            (1, 1) => {
                                // Look through indexes 0-2 in first 3 row
                                (0 as usize..=2 as usize).into_par_iter().for_each(|r| {
                                    // Look throguh indexes 0-2 in first 3 columns
                                    (0 as usize..=2 as usize).into_par_iter().for_each(|c| {
                                        // Insert these indexes into a DashSet for further processing
                                        impossiblepos.insert((r, c));
                                    });
                                });
                            }
                            // Block 2
                            (1, 2) => {
                                (0 as usize..=2 as usize).into_par_iter().for_each(|r| {
                                    (3 as usize..=5 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            // Block 3
                            (1, 3) => {
                                (0 as usize..=2 as usize).into_par_iter().for_each(|r| {
                                    (6 as usize..=8 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            // Block 4
                            (2, 1) => {
                                (3 as usize..=5 as usize).into_par_iter().for_each(|r| {
                                    (0 as usize..=2 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            // Block 5
                            (2, 2) => {
                                (3 as usize..=5 as usize).into_par_iter().for_each(|r| {
                                    (3 as usize..=5 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            // Block 6
                            (2, 3) => {
                                (3 as usize..=5 as usize).into_par_iter().for_each(|r| {
                                    (6 as usize..=8 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            // Block 7
                            (3, 1) => {
                                (6 as usize..=8 as usize).into_par_iter().for_each(|r| {
                                    (0 as usize..=2 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            // Block 8
                            (3, 2) => {
                                (6 as usize..=8 as usize).into_par_iter().for_each(|r| {
                                    (3 as usize..=5 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            // Block 9
                            (3, 3) => {
                                (6 as usize..=8 as usize).into_par_iter().for_each(|r| {
                                    (6 as usize..=8 as usize).into_par_iter().for_each(|c| {
                                        impossiblepos.insert((r as usize, c as usize));
                                    });
                                });
                            }
                            _ => {}
                        }
                    });
                });
            });
            // let mut falsenum: usize = 0;
            // for i in 0..9 {
            //     falsenum += boardbools[i]
            //         .par_iter()
            //         .enumerate()
            //         .positions(|(j, x)| x == &false && !impossibleposvec.contains(&(i, j)))
            //         .count();
            //   }
            //   if impossibleposvec.len() + falsenum == 81 {
            //       numtoremove.insert(
            //        available
            //            .par_iter()
            //            .position_any(|x| x == &numberon)
            //            .unwrap(),
            //    );
            // }

            // At this point, we start to actually insert numbers into the board based on the places where the number cannot go
            rayon::scope(|s| {
                // Analyze rows
                s.spawn(|_| {
                    // Iterate through each row
                    (0 as usize..9 as usize).into_par_iter().for_each(|row| {
                        // We use this variable to determine where in the row it is not possible to place the number
                        let mut rowabs: [bool; 9] = [false; 9];
                        // Iterate through each impossible possition in the row
                        impossiblepos.iter().filter(|i| i.0 == row).for_each(|pos| {
                            // Add the indexes to the list
                            rowabs[pos.1] = true;
                        });
                        // Iterate where there are already numbers in the row
                        boardbools[row].iter().enumerate().for_each(|(i, index)| {
                            // If there is a number already in the row
                            if index.clone() {
                                // Add it's position to the index
                                rowabs[i] = true;
                            }
                        });
                        // If there is 1 single empty space in the row that is not marked by a
                        // number or an impossible position
                        if rowabs.into_par_iter().positions(|x| x == &true).count() == 8 {
                            // Add the number being processed into this space
                            additions.insert((
                                (row, rowabs.into_par_iter().position_any(|x| !x).unwrap()),
                                numberon,
                            ));
                        }
                    });
                });
                // Analyze columns
                s.spawn(|_| {
                    (0 as usize..9 as usize).into_par_iter().for_each(|column| {
                        let mut rowabs: [bool; 9] = [false; 9];
                        impossiblepos
                            .iter()
                            .filter(|i| i.1 == column)
                            .for_each(|pos| {
                                rowabs[pos.0] = true;
                            });
                        boardbools.iter().enumerate().for_each(|(i, list)| {
                            if list[column] {
                                rowabs[i] = true;
                            }
                        });
                        if rowabs.into_par_iter().positions(|x| x == &true).count() == 8 {
                            additions.insert((
                                (rowabs.into_par_iter().position_any(|x| !x).unwrap(), column),
                                numberon,
                            ));
                        }
                    });
                });
            });
        });
        // If there is nothing else to do then stop recursing
        if additions.iter().count() == 0 {
            recurse = false;
        } else {
            // If there is still more to do, dump our previous iteration onto the board and repeat
            // the loop
            additions.iter().for_each(|i| {
                board[i.0 .0][i.0 .1] = i.1;
                boardbools[i.0 .0][i.0 .1] = true;
            });
        }
        /*
         * At this point we would remove the numbers from the iterator where there are no more
         * possible squares that it could go, but I couldn't find a way to do this that would
         * introduce significant performance improvements
         */
        // for i in numtoremove {
        //     available.remove(i);
        // }
    }

    // At this point the puzzle is solved
    let end = time.elapsed();

    // The rest of the code simply displays the puzzle to the screen
    draw!(terminal, board);

    let (tx, rx) = crossbeam_channel::unbounded();

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

    loop {
        match rx.recv().unwrap() {
            event => match event.code {
                event::KeyCode::Char('q') | event::KeyCode::Enter => {
                    disable_raw_mode().unwrap();
                    execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
                    terminal.show_cursor().unwrap();
                    println!("Solved in: {:?} millis", end.as_millis());
                    break;
                }
                _ => {}
            },
        };
    }
}
