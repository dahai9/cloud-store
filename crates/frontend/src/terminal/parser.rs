use crate::terminal::cell::{CellAttributes, Color};
use crate::terminal::grid::Grid;
use vte::{Params, Perform};

pub struct Terminal {
    pub grid: Grid,
    pub attrs: CellAttributes,
}

impl Terminal {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid: Grid::new(rows, cols),
            attrs: CellAttributes::default(),
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.grid.resize(rows, cols);
    }
}

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        self.grid.put_char(c, self.attrs);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.grid.new_line(),
            b'\r' => self.grid.carriage_return(),
            b'\x08' => self.grid.backspace(), // Backspace
            b'\x07' => { /* Bell - ignore for now */ }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            'm' => {
                let mut flat_params = Vec::<u16>::new();
                for group in params {
                    if group.is_empty() {
                        flat_params.push(0);
                    } else {
                        flat_params.extend(group.iter().copied());
                    }
                }

                let mut i = 0usize;
                while i < flat_params.len() {
                    match flat_params[i] {
                        0 => self.attrs = CellAttributes::default(),
                        1 => self.attrs.bold = true,
                        3 => self.attrs.italic = true,
                        4 => self.attrs.underline = true,
                        7 => self.attrs.inverse = true,
                        22 => self.attrs.bold = false,
                        23 => self.attrs.italic = false,
                        24 => self.attrs.underline = false,
                        27 => self.attrs.inverse = false,
                        30 => self.attrs.fg = Color::Black,
                        31 => self.attrs.fg = Color::Red,
                        32 => self.attrs.fg = Color::Green,
                        33 => self.attrs.fg = Color::Yellow,
                        34 => self.attrs.fg = Color::Blue,
                        35 => self.attrs.fg = Color::Magenta,
                        36 => self.attrs.fg = Color::Cyan,
                        37 => self.attrs.fg = Color::White,
                        39 => self.attrs.fg = Color::Default,
                        40 => self.attrs.bg = Color::Black,
                        41 => self.attrs.bg = Color::Red,
                        42 => self.attrs.bg = Color::Green,
                        43 => self.attrs.bg = Color::Yellow,
                        44 => self.attrs.bg = Color::Blue,
                        45 => self.attrs.bg = Color::Magenta,
                        46 => self.attrs.bg = Color::Cyan,
                        47 => self.attrs.bg = Color::White,
                        49 => self.attrs.bg = Color::Default,
                        90 => self.attrs.fg = Color::BrightBlack,
                        91 => self.attrs.fg = Color::BrightRed,
                        92 => self.attrs.fg = Color::BrightGreen,
                        93 => self.attrs.fg = Color::BrightYellow,
                        94 => self.attrs.fg = Color::BrightBlue,
                        95 => self.attrs.fg = Color::BrightMagenta,
                        96 => self.attrs.fg = Color::BrightCyan,
                        97 => self.attrs.fg = Color::BrightWhite,
                        100 => self.attrs.bg = Color::BrightBlack,
                        101 => self.attrs.bg = Color::BrightRed,
                        102 => self.attrs.bg = Color::BrightGreen,
                        103 => self.attrs.bg = Color::BrightYellow,
                        104 => self.attrs.bg = Color::BrightBlue,
                        105 => self.attrs.bg = Color::BrightMagenta,
                        106 => self.attrs.bg = Color::BrightCyan,
                        107 => self.attrs.bg = Color::BrightWhite,
                        38 => {
                            if i + 2 < flat_params.len() && flat_params[i + 1] == 5 {
                                self.attrs.fg = Color::Ansi256(flat_params[i + 2] as u8);
                                i += 2;
                            } else if i + 4 < flat_params.len() && flat_params[i + 1] == 2 {
                                self.attrs.fg = Color::Rgb(
                                    flat_params[i + 2] as u8,
                                    flat_params[i + 3] as u8,
                                    flat_params[i + 4] as u8,
                                );
                                i += 4;
                            }
                        }
                        48 => {
                            if i + 2 < flat_params.len() && flat_params[i + 1] == 5 {
                                self.attrs.bg = Color::Ansi256(flat_params[i + 2] as u8);
                                i += 2;
                            } else if i + 4 < flat_params.len() && flat_params[i + 1] == 2 {
                                self.attrs.bg = Color::Rgb(
                                    flat_params[i + 2] as u8,
                                    flat_params[i + 3] as u8,
                                    flat_params[i + 4] as u8,
                                );
                                i += 4;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            'A' => {
                // Cursor Up
                let count = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(1);
                self.grid.move_cursor(-(count as i32), 0);
            }
            'B' => {
                // Cursor Down
                let count = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(1);
                self.grid.move_cursor(count as i32, 0);
            }
            'C' => {
                // Cursor Forward
                let count = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(1);
                self.grid.move_cursor(0, count as i32);
            }
            'D' => {
                // Cursor Backward
                let count = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(1);
                self.grid.move_cursor(0, -(count as i32));
            }
            'H' | 'f' => {
                // Cursor Position
                let mut iter = params.iter();
                let row = iter
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(1)
                    .saturating_sub(1);
                let col = iter
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.grid.set_cursor(row as usize, col as usize);
            }
            'G' => {
                let col = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .copied()
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.grid.set_cursor_col(col as usize);
            }
            'd' => {
                let row = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .copied()
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.grid.set_cursor_row(row as usize);
            }
            's' => self.grid.save_cursor(),
            'u' => self.grid.restore_cursor(),
            'J' => {
                // Erase in Display
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(0);
                self.grid.erase_in_display(mode);
            }
            'K' => {
                // Erase in Line
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .cloned()
                    .unwrap_or(0);
                self.grid.erase_in_line(mode);
            }
            'X' => {
                let count = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .copied()
                    .unwrap_or(1) as usize;
                self.grid.erase_chars(count.max(1));
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => self.grid.save_cursor(),
            b'8' => self.grid.restore_cursor(),
            _ => {}
        }
    }
}
