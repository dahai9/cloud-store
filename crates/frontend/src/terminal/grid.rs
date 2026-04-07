use crate::terminal::cell::{Cell, CellAttributes};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub rows: usize,
    pub cols: usize,
    pub lines: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub saved_cursor_row: usize,
    pub saved_cursor_col: usize,
}

#[allow(dead_code)]
impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        let lines = vec![vec![Cell::default(); cols]; rows];
        Self {
            rows,
            cols,
            lines,
            cursor_row: 0,
            cursor_col: 0,
            saved_cursor_row: 0,
            saved_cursor_col: 0,
        }
    }

    pub fn put_char(&mut self, c: char, attrs: CellAttributes) {
        if self.cursor_col >= self.cols {
            self.new_line();
        }

        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows - 1;
        }

        self.lines[self.cursor_row][self.cursor_col] = Cell { c, attrs };
        self.cursor_col += 1;
    }

    pub fn new_line(&mut self) {
        self.cursor_col = 0;
        self.cursor_row += 1;
        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows - 1;
        }
    }

    pub fn scroll_up(&mut self) {
        self.lines.remove(0);
        self.lines.push(vec![Cell::default(); self.cols]);
    }

    pub fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn clear_screen(&mut self) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                self.lines[row][col] = Cell::default();
            }
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.rows - 1);
        self.cursor_col = col.min(self.cols - 1);
    }

    pub fn set_cursor_row(&mut self, row: usize) {
        self.cursor_row = row.min(self.rows - 1);
    }

    pub fn set_cursor_col(&mut self, col: usize) {
        self.cursor_col = col.min(self.cols - 1);
    }

    pub fn move_cursor(&mut self, row_delta: i32, col_delta: i32) {
        let new_row = (self.cursor_row as i32 + row_delta)
            .max(0)
            .min(self.rows as i32 - 1) as usize;
        let new_col = (self.cursor_col as i32 + col_delta)
            .max(0)
            .min(self.cols as i32 - 1) as usize;
        self.cursor_row = new_row;
        self.cursor_col = new_col;
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor_row = self.cursor_row;
        self.saved_cursor_col = self.cursor_col;
    }

    pub fn restore_cursor(&mut self) {
        self.cursor_row = self.saved_cursor_row;
        self.cursor_col = self.saved_cursor_col;
    }

    pub fn erase_in_line(&mut self, mode: u16) {
        match mode {
            0 => {
                // From cursor to end of line
                for col in self.cursor_col..self.cols {
                    self.lines[self.cursor_row][col] = Cell::default();
                }
            }
            1 => {
                // From start of line to cursor
                for col in 0..=self.cursor_col.min(self.cols - 1) {
                    self.lines[self.cursor_row][col] = Cell::default();
                }
            }
            2 => {
                // Entire line
                for col in 0..self.cols {
                    self.lines[self.cursor_row][col] = Cell::default();
                }
            }
            _ => {}
        }
    }

    pub fn erase_chars(&mut self, count: usize) {
        let start = self.cursor_col.min(self.cols);
        let end = (start + count).min(self.cols);
        for col in start..end {
            self.lines[self.cursor_row][col] = Cell::default();
        }
    }

    pub fn delete_chars(&mut self, count: usize) {
        if self.cursor_col >= self.cols {
            return;
        }
        let count = count.min(self.cols - self.cursor_col);
        let line = &mut self.lines[self.cursor_row];
        // Shift everything left
        for col in self.cursor_col..(self.cols - count) {
            line[col] = line[col + count];
        }
        // Fill the rest with default
        for item in line.iter_mut().skip(self.cols - count) {
            *item = Cell::default();
        }
    }

    pub fn insert_chars(&mut self, count: usize) {
        if self.cursor_col >= self.cols {
            return;
        }
        let count = count.min(self.cols - self.cursor_col);
        let line = &mut self.lines[self.cursor_row];
        // Shift everything right
        for col in (self.cursor_col..(self.cols - count)).rev() {
            line[col + count] = line[col];
        }
        // Fill the gap with default
        for item in line.iter_mut().skip(self.cursor_col).take(count) {
            *item = Cell::default();
        }
    }

    pub fn delete_lines(&mut self, count: usize) {
        if self.cursor_row >= self.rows {
            return;
        }
        let count = count.min(self.rows - self.cursor_row);
        for row in self.cursor_row..(self.rows - count) {
            self.lines.swap(row, row + count);
        }
        for row in (self.rows - count)..self.rows {
            for col in 0..self.cols {
                self.lines[row][col] = Cell::default();
            }
        }
    }

    pub fn insert_lines(&mut self, count: usize) {
        if self.cursor_row >= self.rows {
            return;
        }
        let count = count.min(self.rows - self.cursor_row);
        for row in (self.cursor_row..(self.rows - count)).rev() {
            self.lines.swap(row, row + count);
        }
        for row in self.cursor_row..(self.cursor_row + count) {
            for col in 0..self.cols {
                self.lines[row][col] = Cell::default();
            }
        }
    }

    pub fn erase_in_display(&mut self, mode: u16) {
        match mode {
            0 => {
                // From cursor to end of display
                self.erase_in_line(0);
                for row in (self.cursor_row + 1)..self.rows {
                    for col in 0..self.cols {
                        self.lines[row][col] = Cell::default();
                    }
                }
            }
            1 => {
                // From start of display to cursor
                for row in 0..self.cursor_row {
                    for col in 0..self.cols {
                        self.lines[row][col] = Cell::default();
                    }
                }
                self.erase_in_line(1);
            }
            2 | 3 => {
                // Entire display
                self.clear_screen();
            }
            _ => {}
        }
    }

    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        if new_rows == 0 || new_cols == 0 {
            return;
        }

        if self.rows == new_rows && self.cols == new_cols {
            return;
        }

        let old_rows = self.rows;
        let old_lines = std::mem::take(&mut self.lines);
        let mut new_lines = vec![vec![Cell::default(); new_cols]; new_rows];

        let rows_to_copy = old_rows.min(new_rows);
        let old_start = old_rows.saturating_sub(rows_to_copy);
        let new_start = new_rows.saturating_sub(rows_to_copy);

        for row_offset in 0..rows_to_copy {
            let old_row = &old_lines[old_start + row_offset];
            let cols_to_copy = old_row.len().min(new_cols);
            new_lines[new_start + row_offset][..cols_to_copy]
                .copy_from_slice(&old_row[..cols_to_copy]);
        }

        self.rows = new_rows;
        self.cols = new_cols;
        self.lines = new_lines;
        self.cursor_row = self.cursor_row.min(new_rows - 1);
        self.cursor_col = self.cursor_col.min(new_cols - 1);
        self.saved_cursor_row = self.saved_cursor_row.min(new_rows - 1);
        self.saved_cursor_col = self.saved_cursor_col.min(new_cols - 1);
    }
}
