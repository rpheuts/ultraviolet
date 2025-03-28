// Amount of extra padding needed for link columns in headers/separators to match
// the natural padding that the terminal's link format (\x1B]8;;URL\x07TEXT\x1B]8;;\x07) provides
const LINK_PADDING: usize = 1;

// ANSI color codes
const BORDER_COLOR: &str = "\x1B[38;5;67m";  // Steel blue
const RESET: &str = "\x1B[0m";

#[derive(Debug, Clone)]
pub enum Cell {
    Text(String),
    Link {
        text: String,
        url: String,
    },
}

impl Cell {
    fn strip_ansi(text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '\x1B' {
                match chars.next() {
                    Some(']') => {
                        if chars.next() == Some('8') && 
                           chars.next() == Some(';') && 
                           chars.next() == Some(';') {
                            // Skip URL until \x07
                            while let Some(c) = chars.next() {
                                if c == '\x07' {
                                    break;
                                }
                            }
                        }
                    }
                    Some('m') => {
                        // Regular ANSI color code, already skipped
                    }
                    _ => {
                        // Skip until we find 'm'
                        while let Some(c) = chars.next() {
                            if c == 'm' {
                                break;
                            }
                        }
                    }
                }
            } else if c != '\x07' { // Don't include the closing \x07
                result.push(c);
            }
        }
        result
    }

    fn truncate_text(text: &str, max_width: usize) -> String {
        let text_width = Self::strip_ansi(text).chars().count();
        if text_width <= max_width {
            text.to_string()
        } else {
            // Take max_width-1 chars to leave room for ellipsis
            let mut result = String::new();
            let mut current_width = 0;
            let mut in_ansi = false;
            let mut ansi_buffer = String::new();

            for c in text.chars() {
                if c == '\x1B' {
                    in_ansi = true;
                    ansi_buffer.push(c);
                } else if in_ansi {
                    ansi_buffer.push(c);
                    if c == 'm' || c == '\x07' {
                        in_ansi = false;
                        result.push_str(&ansi_buffer);
                        ansi_buffer.clear();
                    }
                } else {
                    if current_width < max_width - 1 {
                        result.push(c);
                        current_width += 1;
                    } else if current_width == max_width - 1 {
                        result.push('…');
                        current_width += 1;
                    } else {
                        break;
                    }
                }
            }
            result
        }
    }

    pub fn width(&self) -> usize {
        match self {
            Cell::Text(text) => Self::strip_ansi(text).chars().count(),
            Cell::Link { text, .. } => Self::strip_ansi(text).chars().count(),
        }
    }

    pub fn render(&self, max_width: Option<usize>) -> String {
        match self {
            Cell::Text(text) => {
                if let Some(max) = max_width {
                    Self::truncate_text(text, max)
                } else {
                    text.clone()
                }
            }
            Cell::Link { text, url } => {
                let display_text = if let Some(max) = max_width {
                    Self::truncate_text(text, max)
                } else {
                    text.clone()
                };
                format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", url, display_text)
            }
        }
    }
}

#[derive(Clone)]
pub struct Table {
    headers: Vec<Cell>,
    rows: Vec<Vec<Cell>>,
    column_widths: Vec<usize>,
    column_max_widths: Vec<Option<usize>>,  // Track max width constraints
    column_alignments: Vec<Alignment>,
    column_has_links: Vec<bool>,  // Track which columns contain link cells
    style: blue_render_core::TableStyle,
}

impl Table {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            column_widths: Vec::new(),
            column_max_widths: Vec::new(),
            column_alignments: Vec::new(),
            column_has_links: Vec::new(),
            style: blue_render_core::TableStyle::Default,
        }
    }

    pub fn set_style(&mut self, style: blue_render_core::TableStyle) {
        self.style = style;
    }

    pub fn set_column_max_width(&mut self, column: usize, max_width: Option<usize>) {
        if column >= self.column_max_widths.len() {
            self.column_max_widths.resize(column + 1, None);
        }
        self.column_max_widths[column] = max_width;
    }

    pub fn set_headers(&mut self, headers: Vec<Cell>) {
        // Update column widths and link tracking
        for (i, cell) in headers.iter().enumerate() {
            let width = cell.width();
            if i >= self.column_widths.len() {
                self.column_widths.push(width);
                self.column_alignments.push(Alignment::Left);
                self.column_has_links.push(matches!(cell, Cell::Link { .. }));
                self.column_max_widths.push(None);
            } else {
                if width > self.column_widths[i] {
                    // Only update width if it doesn't exceed max_width
                    if let Some(max) = self.column_max_widths[i] {
                        self.column_widths[i] = width.min(max);
                    } else {
                        self.column_widths[i] = width;
                    }
                }
                self.column_has_links[i] |= matches!(cell, Cell::Link { .. });
            }
        }
        self.headers = headers;
    }

    pub fn set_column_alignment(&mut self, column: usize, alignment: Alignment) {
        if column < self.column_alignments.len() {
            self.column_alignments[column] = alignment;
        }
    }

    pub fn add_row(&mut self, row: Vec<Cell>) {
        // Update column widths and link tracking
        for (i, cell) in row.iter().enumerate() {
            let width = cell.width();
            if i >= self.column_widths.len() {
                self.column_widths.push(width);
                self.column_alignments.push(Alignment::Left);
                self.column_has_links.push(matches!(cell, Cell::Link { .. }));
                self.column_max_widths.push(None);
            } else {
                if width > self.column_widths[i] {
                    // Only update width if it doesn't exceed max_width
                    if let Some(max) = self.column_max_widths[i] {
                        self.column_widths[i] = width.min(max);
                    } else {
                        self.column_widths[i] = width;
                    }
                }
                self.column_has_links[i] |= matches!(cell, Cell::Link { .. });
            }
        }
        self.rows.push(row);
    }

    fn render_separator(&self, left: &str, middle: &str, right: &str) -> String {
        let parts: Vec<String> = self.column_widths.iter()
            .enumerate()
            .map(|(i, width)| {
                // Add extra padding for link columns to match the terminal's link format padding
                let extra = if self.column_has_links[i] { LINK_PADDING } else { 0 };
                "─".repeat(*width + 2 + extra)  // Add 2 for standard padding
            })
            .collect();
        
        format!("{}{}{}{}{}", BORDER_COLOR, left, parts.join(middle), right, RESET)
    }

    fn pad_cell(text: &str, width: usize, align: Alignment) -> String {
        let text_width = Cell::strip_ansi(text).chars().count();
        let padding = if text_width >= width {
            0
        } else {
            width - text_width
        };

        match align {
            Alignment::Left => format!(" {}{} ", text, " ".repeat(padding)),
            Alignment::Right => format!(" {}{} ", " ".repeat(padding), text),
            Alignment::Center => {
                let left = padding / 2;
                let right = padding - left;
                format!(" {}{}{} ", " ".repeat(left), text, " ".repeat(right))
            }
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        // Top border
        if matches!(self.style, blue_render_core::TableStyle::Default) {
            output.push_str(&self.render_separator("┌", "┬", "┐"));
            output.push('\n');
        }

        // Headers
        if !self.headers.is_empty() {
            let header_cells: Vec<String> = self.headers.iter().enumerate()
                .map(|(i, cell)| {
                    let text = cell.render(self.column_max_widths[i]);
                    // Add extra padding for link columns since headers don't have the natural link padding
                    let extra = if self.column_has_links[i] { LINK_PADDING } else { 0 };
                    Self::pad_cell(&text, self.column_widths[i] + extra, Alignment::Left)
                })
                .collect();

            output.push_str(&format!("{}│{}", BORDER_COLOR, header_cells.iter()
                .map(|cell| format!("{}{}", RESET, cell))
                .collect::<Vec<_>>()
                .join(&format!("{}│{}", BORDER_COLOR, RESET))));
            output.push_str(&format!("{}│{}", BORDER_COLOR, RESET));
            output.push('\n');

            // Header separator
            if matches!(self.style, blue_render_core::TableStyle::Default) {
                output.push_str(&self.render_separator("├", "┼", "┤"));
                output.push('\n');
            }
        }

        // Data rows
        for (i, row) in self.rows.iter().enumerate() {
            let cells: Vec<String> = row.iter().enumerate()
                .map(|(i, cell)| {
                    let text = cell.render(self.column_max_widths[i]);
                    // No extra padding needed for data cells since link format provides its own
                    Self::pad_cell(&text, self.column_widths[i], self.column_alignments[i])
                })
                .collect();

            output.push_str(&format!("{}│{}", BORDER_COLOR, cells.iter()
                .map(|cell| format!("{}{}", RESET, cell))
                .collect::<Vec<_>>()
                .join(&format!("{}│{}", BORDER_COLOR, RESET))));
            output.push_str(&format!("{}│{}", BORDER_COLOR, RESET));
            output.push('\n');

            // Row separator (except after last row)
            if i < self.rows.len() - 1 && matches!(self.style, blue_render_core::TableStyle::Default) {
                output.push_str(&self.render_separator("├", "┼", "┤"));
                output.push('\n');
            }
        }

        // Bottom border
        if matches!(self.style, blue_render_core::TableStyle::Default) {
            output.push_str(&self.render_separator("└", "┴", "┘"));
            output.push('\n');
        }

        output
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
}
