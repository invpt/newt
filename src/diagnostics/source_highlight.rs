use inline_colorization::*;

pub struct SourceHighlight<'a> {
    start_row: usize,
    start_col: usize,
    lines: Vec<LineHighlight<'a>>,
}

impl<'a> SourceHighlight<'a> {
    pub fn new(text: &'a str, range: std::ops::Range<usize>) -> Self {
        let mut lines = vec![];

        // row and col are the row and col of the error
        let mut start_row = 0usize;
        let mut start_col = 0usize;

        // row_end and row_start are used
        // to print the line that the erroring
        // code is on
        let mut highlight_start = 0usize;
        let mut highlight_end = 0usize;
        let mut row_start = 0usize;

        // used to facilitate finding
        // row_end once we have found the
        // row:col start of the error
        let mut is_inner_row = false;
        let mut is_final_row = false;

        let mut all_whitespace = std::usize::MAX;

        let mut row_whitespace = 0usize;
        let mut row_whitespace_over = false;
        for (idx, ch) in text.char_indices() {
            if !is_inner_row && idx >= range.start {
                highlight_start = idx;
                is_inner_row = true;
            }

            if !is_final_row && idx >= range.end {
                highlight_end = idx;
                is_final_row = true;
            }

            if ch == '\n' {
                let row_end = idx;
                if is_inner_row || is_final_row {
                    if row_whitespace < all_whitespace {
                        all_whitespace = row_whitespace
                    }

                    if is_inner_row && !is_final_row {
                        highlight_end = row_end;
                    }

                    // if this line isn't completely whitespace
                    if row_whitespace != row_end - row_start {
                        lines.push(LineHighlight::new(
                            &text[row_start..row_end],
                            highlight_start - row_start..highlight_end - row_start,
                        ))
                    }

                    highlight_start = idx + 1;

                    if is_final_row {
                        break;
                    }
                } else {
                    start_row += 1;
                    start_col = 0;
                }

                row_start = idx + 1;
                row_whitespace = 0;
                row_whitespace_over = false;
            } else {
                if !is_inner_row {
                    start_col += 1;
                }

                match ch {
                    ch if ch.is_ascii_whitespace() && !row_whitespace_over => row_whitespace += 1,
                    _ => row_whitespace_over = true,
                }
            }
        }

        for line in &mut lines {
            line.line = &line.line[all_whitespace..];
            line.highlight.start = line.highlight.start.saturating_sub(all_whitespace);
            line.highlight.end = line.highlight.end.saturating_sub(all_whitespace);
        }

        Self {
            start_row,
            start_col,
            lines,
        }
    }

    pub fn start_row(&self) -> usize {
        self.start_row
    }

    pub fn start_col(&self) -> usize {
        self.start_col
    }
}

impl<'a> std::fmt::Display for SourceHighlight<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MAX_NUM_LINES: usize = 10;
        const NUM_LINES_UNSKIPPED_START: usize = 3;
        const NUM_LINES_UNSKIPPED_END: usize = 3;

        fn write_lines(f: &mut std::fmt::Formatter, lines: &[LineHighlight]) -> std::fmt::Result {
            for (i, line) in lines.iter().enumerate() {
                if i != lines.len() - 1 {
                    writeln!(f, "{}", line)?
                } else {
                    write!(f, "{}", line)?
                }
            }

            Ok(())
        }

        if self.lines.len() >= MAX_NUM_LINES {
            write_lines(f, &self.lines[0..NUM_LINES_UNSKIPPED_START])?;

            writeln!(f)?;

            writeln!(
                f,
                "\n   --- skipping {} lines ---\n",
                self.lines.len() - (NUM_LINES_UNSKIPPED_START + NUM_LINES_UNSKIPPED_END)
            )?;

            write_lines(f, &self.lines[self.lines.len() - NUM_LINES_UNSKIPPED_END..])?;
        } else {
            write_lines(f, &self.lines)?
        }

        Ok(())
    }
}

struct LineHighlight<'a> {
    line: &'a str,
    highlight: std::ops::Range<usize>,
}

impl<'a> LineHighlight<'a> {
    fn new(line: &'a str, highlight: std::ops::Range<usize>) -> Self {
        Self { line, highlight }
    }
}

impl<'a> std::fmt::Display for LineHighlight<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, " {color_blue}|{color_reset} {}", self.line)?;

        let mut whitespace_over = false;

        write!(f, " {color_blue}|{color_reset} ")?;

        for (i, ch) in (0..self.highlight.end).zip(self.line.chars()) {
            if !whitespace_over {
                whitespace_over = !ch.is_ascii_whitespace()
            }

            if i < self.highlight.start {
                write!(f, " ")?
            } else if !whitespace_over {
                write!(f, "{style_bold}{color_yellow}^{color_reset}{style_reset}")?
            } else {
                write!(f, "{style_bold}{color_yellow}^{color_reset}{style_reset}")?
            }
        }

        Ok(())
    }
}
